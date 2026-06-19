# Plan/Apply Redesign — Note 01: Mapping Terraform onto altium-cli

**Session date:** 2026-06-18

This note maps each Terraform concept onto the altium-cli spec pipeline, decides
which ones we need, and identifies what already exists vs. what must be built.
Builds on note `00`.

---

## 1. Concept mapping table

| Terraform                         | altium-cli equivalent                                      | Status today |
| --------------------------------- | --------------------------------------------------------- | ------------ |
| `.tf` config (desired state)      | `*-spec` file (the SpecModel after compile)               | ✅ exists    |
| Real infrastructure               | The Altium document (`.SchDoc`, `.PcbDoc`, …)             | ✅ exists    |
| Provider                          | The per-domain executor/reconciler/dump code              | ✅ exists (split) |
| Resource type (`aws_instance`)    | `EntityKind` (Component, Pad, Track, Net, …)              | ✅ exists    |
| Resource address (`type.name`)    | **(missing)** stable per-entity identity/path             | ⚠️ ad-hoc, name-based |
| Refresh (read real → state)       | the read half of `reconcile` (parse doc into comparables) | ⚠️ uses a *different* schema than write |
| State file (address ↔ real id)    | **(missing)** — see §4                                     | ❌ none      |
| `terraform plan`                  | `compile plan` / `dump plan` (reconcile → ECO)            | ✅ exists (add/update/unchanged only) |
| Plan actions create/update/delete/replace/no-op | `EntityChange::{Add,Update,Unchanged}`     | ⚠️ no delete, no replace |
| Saved plan (`-out`)               | **(missing)** — `plan` only prints                        | ❌ none      |
| `terraform apply`                 | `compile apply` / `dump apply` (executor)                | ⚠️ separate engine from plan |
| `terraform destroy`               | **(missing)**                                             | ❌ none      |
| `-target=addr`                    | **(missing)** scoped plan/apply                           | ❌ none      |

**Key divergence from Terraform: this model is *bidirectional* (note 05).**
Terraform has one source of truth (config) and one target (infra). Here *both*
the `-spec` and the document are editable, so there are **two** compile/apply
pairs: `compile` (spec→doc) and `dump` (doc→spec). Each direction is a Terraform
provider in its own right — the table above describes the `compile` (spec→doc)
direction; `dump` (doc→spec) is the mirror, with the `-spec` file as the "real
infrastructure" being reconciled. `dump` is the bidirectional analog of
`terraform import`, except it is a full plan/apply pair (review + merge), not a
one-shot.

The two ❌/⚠️ clusters that matter most: **resource address + state** (identity),
and **plan/apply unification + full lifecycle** (delete/replace/saved plan) —
both shared across the two directions.

---

## 2. The single most important principle: plan == apply

Terraform's defining property is that **`apply` executes the exact plan**.
The plan is not advisory; it is the instruction set. This guarantees:

- What you review is what runs (no plan/apply drift, our §2/fix-#19 bug class).
- A single place defines "what a change is."

**Target architecture:** the ECO becomes the *executable* artifact.

```
SpecModel  ─┐
            ├─► reconcile() ─► ECO (change set) ─► execute_eco() ─► mutated doc ─► save
current doc ┘                     │
                                  └─► render_text / render_json  (this is `plan`)
```

`plan`  = `reconcile()` then render (no execute).
`apply` = `reconcile()` then `execute_eco()` then save. (Or: `apply --plan <saved-eco>`.)

This holds for *both* directions: `dump apply` runs the same reconcile→execute,
with the `-spec` as the target artifact (note 05 §2).

Today `compile apply` (the old root `apply`) calls `executor.rs` directly from
the SpecModel and never builds an
ECO. The redesign routes apply *through* the ECO. `executor.rs` becomes the
implementation of `execute_eco` (one `EntityChange` at a time) rather than a
parallel whole-model walker.

### Why this also fixes the "plan over-reports" problem indirectly

Once apply consumes the ECO, any entity the reconciler marks `Unchanged` is, by
construction, not touched by apply. The reconciler becomes the *only* definition
of change, so "plan says unchanged but apply mutates" is impossible.

---

## 3. Resource address / identity (the hard part)

Terraform needs a stable address per resource so plan can answer "is this the
same resource as last time?" We need the same to answer "is this spec entity the
same as this document entity?"

### What identity anchors already exist

- **SchLib/SchDoc:** `UniqueId` (per-object), and the spec already emits
  `#[annotation(<short-id>)]` blocks. The reconciler already matches SchLib
  graphics by `unique_id` (`reconciler.rs:948`).
- **PcbDoc/PcbLib:** primitives carry `UniqueId` in many cases; components have
  designators; footprints have names.
- **Spec annotations:** `annotation.rs` `generate_short_id()` already mints
  stable short IDs and dump already writes them before each block.

### The gap

Identity is applied **inconsistently** and **by name where no UniqueId is used**:

- Components/footprints/pads/nets → matched by name/designator.
- Nameless PCB board primitives (tracks, fills, arcs, regions) → **no match
  key at all**, so PcbDoc plan reports every one as an ADD (spec-problems §7).

### Proposal: a uniform `ResourceAddress`

Define an address that every entity in every domain can produce, in priority
order:

1. **Explicit annotation ID** if the spec block carries `#[annotation(id)]`
   (authoritative, survives renames). ← this is our "state binding."
2. **UniqueId** from the document object, when present.
3. **Natural key** — domain-specific tuple that is stable under no-op edits:
   - component: `designator`
   - pad: `(footprint, pad_name)`
   - net: `name`
   - track: `(layer, start, end, width)` structural key (content-hash) as a
     last resort for nameless primitives.

The address is what plan diffs on and what the ECO records as `identity`. For
nameless primitives, the structural key means "same geometry = same resource =
no-op," which is exactly what we want for the PcbDoc false-ADD problem: an
unchanged track hashes identically on both sides → `Unchanged`.

This is *lighter* than Terraform state (we re-derive identity from content each
run) but sufficient because the document **is** the state — see §4.

---

## 4. Do we need a separate state file?

Terraform keeps state because the real provider only exposes opaque IDs and you
can't re-derive the config↔resource mapping cheaply. **Our situation is
different: the document is fully readable and round-trippable.** We can refresh
(read the whole doc) on every plan. So:

- **MVP: stateless / document-as-state.** Re-read the target doc each `plan`/
  `apply`; derive identity via the `ResourceAddress` scheme (§3). No `.tfstate`.
  Annotation IDs embedded in the spec carry the only "sticky" binding we need
  (rename tracking).
- **Future: optional sidecar state** only if we find edits that can't be
  re-derived — e.g. tracking a rename where neither the annotation ID nor a
  natural key survived. Defer until a concrete case demands it (same reasoning
  as spec-problems decision-#1 "build A later").

**Conclusion:** no state file in v1. The annotation-ID-in-spec + read-the-doc
approach is the pragmatic analog and avoids a whole class of state-drift bugs.

---

## 5. Full lifecycle: the action set

Extend `EntityChange` to Terraform's action set:

| Action       | Meaning                                              | Needed for |
| ------------ | --------------------------------------------------- | ---------- |
| `Add`        | in spec, not in doc → create                         | ✅ have    |
| `Update`     | in both, props differ → in-place modify              | ✅ have    |
| `Unchanged`  | in both, identical → no-op                           | ✅ have    |
| `Delete`     | in doc, not in spec → remove                          | ❌ **add** |
| `Replace`    | identity-changing edit → delete + recreate           | ❌ add (later) |

`Delete` requires a policy decision: **does apply remove doc entities absent
from the spec?** Terraform: yes (config is authoritative). But our specs are
sometimes *partial* (e.g. PcbDoc spec omits layer stack — STATUS.md). So:

- Default to **authoritative within a managed scope** but allow **partial
  specs** via an explicit opt-out (a per-domain or per-section "manage only what
  the spec mentions" mode). Terraform's `-target` and `ignore_changes` are the
  precedent. See note `03` §4 for the proposed flags.

`Replace` matters when an entity's identity (address) changes — e.g. a track's
geometry changes so its structural key changes. With content-hash identity that
naturally reads as Delete+Add; whether to *call* it Replace is cosmetic for now.

---

## 6. Summary of required building blocks

1. `ResourceAddress` abstraction (annotation-id → unique-id → natural/structural
   key) shared by reconciler and executor. **(new)**
2. `EntityChange::Delete` (+ later `Replace`); `EcoSummary` deletes count;
   renderers. **(extend eco.rs)**
3. `execute_eco(doc, &ECO)` so apply runs the plan, not the SpecModel.
   `executor.rs` refactored to per-change appliers. **(refactor)**
4. Reconciler emits Deletes by walking doc entities not matched by any spec
   entity. **(extend reconciler.rs)**
5. Partial-spec / authoritative-scope policy + flags. **(new)**
6. (Optional) saved-plan serialization: ECO is already `Serialize`; add an
   `apply <eco.json>` path. **(small)**

Next: note `02` shows how this framing forces and informs SchDoc problem #1;
note `03` proposes the concrete per-document-type CLI.
