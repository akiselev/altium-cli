# Plan/Apply Redesign — Note 02: How Terraform framing resolves Problem #1

**Session date:** 2026-06-18

`docs/spec-problems.md` Decision #1 is the SchDoc "inline children" fork: dump
emits full inline geometry (pins/graphics/parameters with absolute coords),
while apply creates components with **empty children** and synthesizes
connectivity from a library symbol (`symbol: $lib.Name` + `pin X -> #NET`). Dump
produces a spec **richer than apply can consume** → 881 SchDoc roundtrip
"failures." The doc framed three options (A materialize inline, B authoring-only
dump, C rich dump + advisory). This note argues the Terraform model **forces the
answer** and reframes it as a schema invariant rather than a product toss-up.

---

## 1. Restating problem #1 in Terraform terms

Terraform's plan works only because **refresh reads real infrastructure into the
same schema the config is written in.** `plan` = diff(config, refreshed-state),
and both sides are the *same resource schema*. If `terraform import` produced a
schema your `.tf` files could not express, plan could never converge — every run
would show spurious diffs forever.

That is *exactly* the SchDoc situation:

- **dump = import/refresh** → produces schema R (Rich: inline children).
- **apply = config materialization** → consumes schema W (library-instance).
- **R ≠ W.** Therefore plan can never reach "no changes" for a dumped sheet.

So problem #1 is not "is SchDoc an authored or inspection format?" (the framing
in spec-problems §6). Under Terraform it becomes a hard invariant:

> **Read schema and write schema MUST be the same schema.** Otherwise plan/apply
> cannot converge, and you do not have a provider — you have two lossy converters
> pointed at each other.

This is also a restatement of the project's own CARDINAL RULE applied at the spec
layer: dump that emits detail apply silently drops is the spec-language analog of
"silently dropping a field on save." We already reject that for the file format
(fail-fast); the spec layer should not get a pass.

---

## 2. Which option survives the invariant

- **Option C (rich dump, compare only apply-consumable subset, inline = advisory).**
  *Rejected by the invariant.* It *institutionalizes* R ≠ W and makes plan lie:
  a user edits an inline child, plan/apply ignore it, file is unchanged. That is
  the plan/apply-drift bug class we are explicitly trying to delete. C is a
  metric-honesty patch, not a provider.

- **Option B (authoring-only dump: emit only `symbol:` + connections).**
  *Satisfies the invariant by shrinking R down to W.* Honest plan/apply by
  construction. **Cost:** dump becomes lossy-by-design — you cannot reconstruct
  a SchDoc without its libraries, and library-less inspection (a core use of
  dump) dies. Also impossible for sheets with primitives not backed by any
  library symbol (free wires, notes, sheet graphics).

- **Option A (teach apply to materialize inline children).**
  *Satisfies the invariant by growing W up to R.* This is the real provider:
  one schema, lossless both directions, SchDoc becomes first-class editable.
  **Cost:** the large executor work spec-problems already flagged (flat-inline →
  OWNERINDEX tree reconstruction, orientation/coordinate handling, merge
  semantics when both inline children *and* `symbol:` appear).

**The Terraform model picks A.** B and C both keep two schemas; only A gives one.
The earlier "C now, A later" recommendation was a *metric* decision; once the
goal is an actual plan/apply provider, A is the only endpoint. C can still be a
*temporary scaffold* (see §4), but it is no longer a destination.

---

## 3. Resolving A's hardest sub-question: two sources of truth

spec-problems §6 worried that inline geometry + a `symbol:` reference creates two
sources of truth that can drift ("a SchDoc component is an *instance* of a
library symbol"). Terraform has a precise answer to "two ways to specify the same
thing": **one is authoritative; the other is a default/template, and the
authoritative one wins deterministically.**

Proposed rule for SchDoc components:

1. **Inline children, when present, are authoritative.** They are the literal
   desired geometry. apply materializes them verbatim (this is the schema-W-grows
   work).
2. **`symbol: $lib.Name` is a *template/generator*,** used only to *produce*
   inline children when none are given (current authoring path). After
   generation the result is just inline children — no longer a live link.
3. **Both present:** `symbol:` seeds, inline overrides per-child by address
   (same `ResourceAddress` scheme from note `01`). This is `terraform`'s
   "module defaults + explicit resource args" pattern.

This keeps the convenient library-instance authoring path *and* gives a single
authoritative representation. The "instance drift" worry dissolves because the
spec, once dumped, no longer claims a live link — it claims concrete geometry.

If a true *live* library link is later desired (re-pull symbol on every apply),
that is a separate, explicit feature (a `link:` mode), not the default — exactly
how Terraform treats data sources vs. resources.

---

## 4. Incremental path (C as scaffold, A as destination)

The invariant says A is the endpoint, but A is large. Sequence it so every step
is shippable and honest:

1. **Cheap, do-regardless:** apply sheet-level document `parameter` blocks
   (CurrentDate, DocumentName, …) — flat key/values, no tree reconstruction.
   (spec-problems §6 "separable sub-piece".)
2. **Make plan honest *now* (scaffold C, but labeled):** until apply can
   materialize inline children, plan must **not** silently treat them as
   no-ops. It should emit them as `Update`/`Add` it *cannot yet execute* and
   `apply` must **hard-error** ("inline child materialization not yet
   implemented for <kind>") rather than silently drop. This preserves fail-fast
   and stops the metric from lying *without* pretending convergence.
   — This is the key correction to spec-problems' "C now": C must **fail loud**,
   not silently compare a subset.
3. **Grow W to R, one child type at a time (A):** pins → net labels/wires/power
   → body graphics → parameters. Each removes one hard-error from step 2 and one
   class of roundtrip failures. The `ResourceAddress` work (note 01 §3) is the
   prerequisite — inline children need stable addresses to diff/merge.
4. **Merge semantics (A finish):** implement the §3 authoritative-inline rule
   for the `symbol:` + inline overlap.

This converts the 881 SchDoc failures from "silently discarded detail" into a
tracked, fail-fast backlog that shrinks monotonically — and at no point does
plan claim a convergence it doesn't have.

---

## 5. Consequence for the other domains

The same invariant audits every domain's dump vs apply:

- **PcbDoc:** dump currently *omits* layer stack and board geometry blocks
  "until the spec compiler supports applying them" (STATUS.md). That is R < W in
  the omitted area, the mirror image of SchDoc. Same invariant: dump should emit
  them and apply should consume them (or plan should fail-loud that they are
  unmanaged), not silently omit.
- **SchLib/PcbLib:** §10 of spec-problems lists silent normalizations (pin
  `show_name`, graphic colors/widths, pad `corner_radius_pct`, …) — each is a
  small R ≠ W gap. The Terraform model says: capture them in the schema or
  fail-loud; do not silently normalize.

So problem #1 is the largest instance of a general invariant the redesign
imposes everywhere: **dump and apply share one schema, and any gap is an explicit
fail-loud TODO, never a silent drop.**

Next: note `03` proposes the concrete per-document-type CLI and flags.
