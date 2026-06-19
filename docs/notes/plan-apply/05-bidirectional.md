# Plan/Apply Redesign — Note 05: Bidirectional editing (CORRECTS notes 01/03)

**Session date:** 2026-06-18

**Correction to the earlier framing.** Notes 00–04 modeled this as *one*
direction: spec = desired state, document = real infrastructure, `apply` pushes
spec→doc and `plan` previews it. That is only half of it.

**The real model is bidirectional.** *Both* the Altium document and the `-spec`
file are first-class editable artifacts, and *each direction* gets its own
plan/apply review gate:

```
        ┌──────────────  apply  ────────────►┐
   .schdoc-spec                            .SchDoc
   (LLM edits here)                    (human edits here, in Altium Designer)
        └◄─────────────   dump   ────────────┘
```

- **spec → doc** (`apply`): an LLM/agent edits the `-spec`; `apply plan` shows
  the ECO of changes to the `.SchDoc`; `apply` commits them.
- **doc → spec** (`dump`): a human edits the `.SchDoc` in Altium (spatial work
  LLMs are bad at); **`dump plan` shows how the `-spec` file would change**;
  `dump apply` commits the new `-spec`.

**Therefore `dump` ALSO needs `plan` and `apply` subcommands**, exactly like
`apply` does. This is the user's correction and it changes the design materially.

The motivation: **LLM agents are weak at spatial/geometric tasks.** Let humans do
the spatial editing in Altium's GUI and let agents work on the textual `-spec`.
The two artifacts must stay in sync, and every sync — in either direction — must
be *reviewable as a diff* before it's written. plan/apply is that review gate.

---

## 1. Two plan/apply pairs, symmetric

| Direction | "plan" shows a diff of… | "apply" writes… | who edits the source |
| --------- | ----------------------- | --------------- | -------------------- |
| **spec → doc** (`apply plan`/`apply`) | the **document** (ECO of Altium objects) | the `.SchDoc` | LLM/agent (textual) |
| **doc → spec** (`dump plan`/`dump apply`) | the **`-spec` file** (diff of spec text/blocks) | the `.schdoc-spec` | human (spatial, in Altium) |

Both are reconcile-then-{render | execute}. The engine is the same shape; only
the *target artifact* differs:

- `apply plan` = reconcile(spec, doc) → ECO over **document entities** → render.
- `dump plan`  = reconcile(doc, existing-spec) → ECO over **spec blocks** → render.

So "plan" is universally "preview the change to the thing apply will write," and
"apply" is "write it." Clean symmetry.

---

## 2. The critical consequence: `dump apply` must MERGE, not overwrite

Today `dump` regenerates the whole `-spec` from scratch every time. For
bidirectional editing that is wrong, because **the existing `-spec` carries
information the document does not**:

- `#[annotation(<short-id>)]` stable IDs (regenerated randomly each dump today —
  spec-problems §2 strips them from the roundtrip diff *because* they churn).
- Human/LLM **comments** (trivia.rs already preserves comments on
  parse→reformat).
- Block **ordering**, formatting, and any spec-only authoring constructs
  (`symbol: $lib.Name` library references, `pin X -> #NET` connections that the
  document represents as concrete wires/labels).

`dump apply` must therefore **reconcile the freshly-read document against the
existing `-spec`** and produce a *minimal edit* of that spec:

- entity changed in Altium → update only the changed properties of its spec
  block, **preserving its annotation ID, comments, and position**.
- entity added in Altium → insert a new spec block (mint a new annotation ID).
- entity deleted in Altium → remove its spec block (with its comments — warn).
- entity unchanged → byte-identical spec block (no churn).

This is the mirror image of what `apply` already does merging into an existing
document. **It needs the same `ResourceAddress` identity layer (note 01 §3)** —
the annotation ID ↔ UniqueId binding is what lets `dump plan` say "U1's `at:`
moved" instead of "U1 block replaced." So `ResourceAddress` is now doubly
load-bearing: it powers identity in *both* directions.

### Annotation IDs become the durable cross-artifact binding

Note 01 §4 said "no state file needed; annotation IDs carry the rename binding."
Bidirectional editing makes that the *core* mechanism, not an aside:
**annotation ID in the spec ↔ UniqueId in the document is the two-way identity
link.** Both `apply` and `dump apply` match on it. This means:

- Dump must **stop regenerating annotation IDs randomly**; it must reuse the ID
  already bound to that document object (persist the ID↔UniqueId mapping — either
  inferred by matching on UniqueId each run, or recorded). This directly retires
  the spec-problems §2 "strip annotations from the diff" hack: once IDs are
  stable, they belong *in* the diff.

---

## 3. Schema symmetry (Problem #1) is now non-negotiable

Note 02 argued read-schema must equal write-schema. Bidirectional editing makes
this **structurally mandatory**, not just desirable:

- `dump` writes schema R; `apply` reads schema W. If R ≠ W, then a human edits
  the doc → `dump apply` writes R into the spec → an LLM edits that spec →
  `apply` can't consume the R parts → the human's edit is silently lost on the
  return trip. **The loop leaks.**

A bidirectional editor with two different schemas is not an editor; it's a
data-loss machine. So Problem #1 Option A (one schema, materialize inline
children) is now a hard prerequisite for SchDoc bidirectional editing — and the
note-02 fail-loud scaffold is the safety net until A is complete: if `apply`
can't yet round-trip a block that `dump` wrote, it must **error**, never silently
drop, so the loop fails loudly instead of leaking.

---

## 4. Conflict handling (both sides edited)

Bidirectional editing introduces the git problem: what if **both** the `.SchDoc`
and the `.schdoc-spec` changed since they were last in sync?

- `apply plan` would show doc changes; `dump plan` would show spec changes; they
  may touch the same entity → conflict.
- v1 stance: **detect and refuse, don't auto-merge.** Each `apply`/`dump apply`
  re-reconciles; if the *other* artifact has drifted from what this side expects,
  warn ("`.SchDoc` has changes not reflected in the spec — run `dump plan`
  first"). This mirrors note 03 §3's stale-saved-plan check, generalized.
- Detecting "the other side drifted" needs a **last-synced baseline**. This is
  the first concrete case (note 01 §4) where a small **sidecar state** may earn
  its keep: record a hash/snapshot of both artifacts at last successful sync, so
  each side can tell "did the other change?" Still optional for a single-author
  workflow, but required for safe concurrent bidirectional editing. *Open
  decision — see §6.*

---

## 5. Revised CLI (see note 03 for the full surface)

Per document type, **two** plan/apply pairs — the verb names the direction.
`compile` = spec → doc (replaces the old root `apply`); `dump` = doc → spec:

```
# spec → doc
altium schdoc compile plan  <spec> [--target doc]                # ECO over the .SchDoc
altium schdoc compile apply <spec> [--target doc] [-o out] [--plan eco.json]

# doc → spec   (now a review gate, merges into the existing spec)
altium schdoc dump plan     <doc>  --spec <spec>                 # ECO over the -spec
altium schdoc dump apply    <doc>  --spec <spec> [-o out] [--plan eco.json]
```

Notes:
- `dump plan <doc> --spec <spec>` reconciles the document against the existing
  spec and prints the **spec-side** ECO (what blocks/props would change in the
  `-spec`). A non-existent `--spec` yields an all-`Add` ECO (first dump).
- `dump apply` always *merges into* the existing spec — there is no overwrite/
  from-scratch dump anymore (§2). First-time dump is the empty-spec degenerate
  case.
- Both directions share: `--json`, `--out <eco.json>` / `--plan <eco.json>`,
  exit codes, and the `ResourceAddress` identity layer.
- The ECO type (`eco.rs`) is reused for both; the renderer labels the target
  artifact ("Document" vs "Spec") in the header.

---

## 6. Impact on the roadmap (amends note 04)

- **Phase 1.1 `ResourceAddress`** is now even more central — it serves both
  directions. Unchanged priority (first).
- **NEW Phase 2.x: stable annotation IDs in dump.** Stop random regeneration;
  bind ID↔UniqueId. Retires the spec-problems §2 annotation-strip hack.
  Prerequisite for meaningful `dump plan` diffs.
- **NEW: `dump plan` / `dump apply` (merge-into-existing-spec).** Mirror of the
  apply-side reconcile, targeting the spec artifact. Depends on 1.1 + stable IDs.
- **Phase 4 (schema symmetry / Problem #1)** is promoted from "quality" to
  "correctness prerequisite for the doc→spec→doc loop" (see §3).
- **NEW open decision:** sidecar last-synced baseline for conflict detection
  (§4) — needed for safe concurrent two-author editing; deferrable for
  single-author.

## 7. Corrections to earlier notes

- **Note 01 §4** ("no state file needed"): still true for v1 single-author, but
  bidirectional concurrent editing makes a small last-synced baseline the first
  real candidate for sidecar state (§4 here).
- **Note 03** (CLI): now revised to the bidirectional surface — every type has
  *two* plan/apply pairs (`compile` for spec→doc, `dump` for doc→spec). See note
  03 for the authoritative command list; §5 here is the SchDoc summary.
- **Note 02**: its invariant is upgraded from "should" to "must" (§3 here).
