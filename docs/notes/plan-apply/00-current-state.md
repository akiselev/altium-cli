# Plan/Apply Redesign — Note 00: Current State of dump/apply/plan

**Session date:** 2026-06-18
**Goal:** Investigate moving the spec dump/apply pipeline to a Terraform-style
`plan` / `apply` model, per document type, where `plan` emits an Engineering
Change Order (ECO) and `apply` executes it. This note records *how things work
today* so the redesign has a faithful baseline. See also `docs/spec-problems.md`
problem #1 (SchDoc inline children), which the Terraform framing forces us to
confront — covered in note `02`.

---

## 1. The four verbs that exist today

The spec pipeline (per `docs/spec-problems.md` §2):

```
dump      (Altium doc → *-spec text)        [dump.rs]
  → parse (text → AST)                       [lexer.rs + parser.rs]
  → compile (AST → SpecModel, typed)         [compiler.rs]
  → { plan:  reconcile SpecModel vs doc → ECO    [reconciler.rs] }
  → { apply: execute  SpecModel onto doc          [executor.rs]   }
```

CLI surface (`crates/altium-cli/src/main.rs`):

| Command  | Direction          | Engine          | Output                        |
| -------- | ------------------ | --------------- | ----------------------------- |
| `dump`   | doc → spec text    | `dump.rs`       | `.{type}-spec` file           |
| `plan`   | spec + target → Δ  | `reconciler.rs` | ECO (text or `--json`)        |
| `apply`  | spec → doc         | `executor.rs`   | written `.{type}` file        |
| `validate` | doc → ok/err     | core parser     | exit code                     |

> **Note (see `05`):** the goal is **bidirectional** — `dump` (doc→spec) must
> *also* gain `plan`/`apply`, because both the document and the `-spec` are
> editable artifacts and each direction needs a review gate. Today `dump` has no
> plan and overwrites the spec wholesale; note 05 covers the correction.

`plan` already follows the Terraform exit-code convention: **exit 1 when the ECO
is non-empty** (drift detected), exit 0 when clean — same as `format --check`
(`run_plan` returns `has_changes`, `main` maps it to `ExitCode`).

Spec file extensions: `.schlib-spec`, `.pcblib-spec`, `.schdoc-spec`,
`.pcbdoc-spec`, `.prjpcb-spec`. Domain is detected from extension
(`detect_spec_domain`).

---

## 2. The core architectural problem: plan and apply are TWO engines

`plan` is computed by `reconciler.rs`; `apply` is executed by `executor.rs`.
**They are independent code paths that must be manually kept in agreement.**

This is exactly the failure mode Terraform's architecture is designed to
prevent: in Terraform, `apply` *executes the plan that was produced*, so the
plan cannot lie about what apply will do. Here they can — and have:

- `spec-problems.md` §4 fix #19 (`diff_pcb_pads`): `plan` compared only
  location/shape/size/hole/plated/layer/rotation, so it reported `Unchanged`
  while `apply` silently mutated `pad_mode`/`mid_*`/`bot_*`/`hole_shape`/
  `slot_size`. Plan said "no change"; apply changed the file. They had to be
  hand-patched back into agreement.

Every spec field added in the future must be wired into **both** engines or they
drift again. This is the single biggest structural argument for the redesign.

---

## 3. The ECO model today (`eco.rs`)

`EngineeringChangeOrder { library_path, spec_path, timestamp, summary, changes }`.

`EntityChange` is the unit of change:

```rust
enum EntityChange {
    Add    { kind, identity, props,        children },
    Update { kind, identity, prop_changes, children },
    Unchanged { kind, identity },
}
```

`EntityKind`: Component, Pin, Parameter, Alias, Graphic, Footprint, Pad, Track,
Via, Arc, Text, Fill, Region, … (one flat enum across all domains).

`EcoSummary` = per-kind counts of `{ adds, updates, unchanged }`.

Rendering: `render_text()` (boxed "ENGINEERING CHANGE ORDER" report, collapses
runs of Unchanged) and `render_json()`.

### Gap A — there is no Delete

**The ECO has no `Delete`/`Destroy` variant.** `grep Delete|Remove|Destroy`
over `eco.rs` and `reconciler.rs` returns nothing. Consequence: a component/pad/
track present in the *target document* but absent from the *spec* is never
reported and never removed. The model is **additive/merge-only**.

For dump→apply-into-new-doc this is invisible (nothing pre-exists to delete).
For "edit an existing board to match a spec" — the actual Terraform use case —
it means the tool cannot converge a document *down* to the spec. Terraform's
whole value is full-lifecycle reconciliation (create + update + **delete** +
replace + no-op). This is a required addition.

### Gap B — identity matching is ad-hoc and name-based

Reconciler matches spec entities to document entities by **name/designator**
(`spec.name`, `var.designator`) and, for SchLib graphics, by **`unique_id`**
(`reconciler.rs:948` matches `g.unique_id() == spec_graphic.unique_id`).

There is **no general identity layer**. The consequences are documented:

- PcbDoc (`spec-problems.md` §7): "the reconciler has no spec↔document identity
  matching and reports all dumped board primitives (tracks, etc.) as ADDs."
  Board primitives (tracks, fills, regions) have no name, so every plan against
  an unchanged board falsely reports a full re-add. `plan` is unusable for
  PcbDoc bodies today.

Terraform solves this with a **state file** that records the binding between a
config address (`aws_instance.web`) and a real resource ID (`i-0abc…`). Our
equivalent identity anchors already exist in the format — `UniqueId`s,
annotation short-IDs — but they are not used as a uniform "resource address."
Note `01` develops this.

---

## 4. Per-document-type apply asymmetry

From `apply_for_model` (`main.rs:1089`):

| Domain | apply into NEW doc                | apply into EXISTING (target) | plan engine            |
| ------ | --------------------------------- | ---------------------------- | ---------------------- |
| SchLib | ✅ `new_blank_ad26` + executor    | ✅ open + executor (merge)   | reconcile / _empty     |
| PcbLib | ✅                                | ✅                           | reconcile / _empty     |
| PrjPcb | ✅                                | ✅                           | reconcile / _empty     |
| SchDoc | ✅ (but lossy — problem #1)       | ✅                           | reconcile / _empty     |
| PcbDoc | ❌ **requires existing target**   | ✅ (+ footprint instantiate) | reconcile (false adds) |

PcbDoc apply *requires* an existing target (`main.rs:1180` bails otherwise) and
runs an extra `instantiate_footprint_primitives` pass that pulls geometry from
sibling `.schdoc-spec` + imported footprints. So PcbDoc is already a "mutate
real infrastructure" model, never a "create from scratch" model — which is
actually *more* Terraform-like than the libs.

SchDoc apply is the problem-#1 case: it creates components with **empty
children** (library-instance authoring model), while dump emits **inline
children** (full geometry). Read schema ≠ write schema. See note `02`.

---

## 5. Roundtrip status recap (from STATUS.md / spec-problems.md)

| Corpus | dump→apply→validate→re-dump | Dominant remaining blocker          |
| ------ | --------------------------- | ----------------------------------- |
| SchLib | 121/126                     | `%UTF8%` trim; 1 dup storage name   |
| PcbLib | 33/40 (0 roundtrip diffs)   | parser gaps; SectionKeys on apply   |
| SchDoc | 67/1226                     | **problem #1** (881) + parser gaps  |
| PcbDoc | dump+plan on 79/132         | no identity matching → false ADDs   |

The two redesign-relevant blockers are **SchDoc problem #1** (read≠write schema)
and **PcbDoc identity matching** (no resource address). Both are *exactly* what
the Terraform model is built to force you to solve.

---

## 6. What "Terraform-style" must add (preview — detailed in 01/03)

1. **One engine.** `apply` executes the change set `plan` produced. Kill the
   reconciler/executor split (or make executor a thin "apply an ECO" driver).
2. **Full lifecycle.** Add `Delete` (and probably `Replace` = delete+create for
   identity-changing edits) to `EntityChange`.
3. **Resource identity / address.** A uniform identity scheme (UniqueId /
   annotation ID / structural key) so plan can match spec↔doc entities across
   all domains, including nameless PCB primitives.
4. **Refresh = read into the same schema as config.** Forces problem #1.
5. **Saved plans (optional but high-value).** `plan -out=file` then
   `apply file` so review and execution are the same artifact.

Next: note `01` maps each Terraform concept onto altium-cli precisely.
