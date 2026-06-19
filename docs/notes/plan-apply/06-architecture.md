# Plan/Apply Redesign — Note 06: Rust architecture & the two ECOs

**Session date:** 2026-06-18 (rev: semantic diff on Rust models, not text; CFB
ECO is a separate agent-facing artifact)

Research into *how* to implement plan/apply as Rust types: what Rust data
structures the diff runs on, and the relationship between the two outputs the
user wants.

**Two distinct, parallel outputs — not two altitudes of one thing:**

1. **Semantic ECO** — the engineering change order. Computed **purely on our Rust
   data structures**, in **every direction** (compile *and* dump). This is the
   user-facing change order and the primary artifact. The spec text and the CFB
   bytes are *just serializations*; the diff never runs on either of them.
2. **Low-level CFB diff** — the serialized-stream/block/byte delta
   (`DiffIssue`). A **separate, more specific artifact for computer agents** to
   programmatically verify file-format support (e.g. "did our serializer write
   the bytes the format expects?", roundtrip checks). **Not** a user ECO, **not**
   nested under the semantic ECO.

Corrects note 06's earlier draft: there is **no `SpecText` diff** and the
semantic ECO is **not** an index into the byte diff — those two ideas are dropped.

Builds on notes `00`–`05`.

---

## 1. The semantic ECO runs on a *common Rust model*, in all directions

The spec file is **just a serialization format**. The Altium document is another
serialization. Both deserialize into Rust data structures. The semantic diff must
therefore be:

> `reconcile(model_a: &M, model_b: &M) -> EngineeringChangeOrder`
>
> where `M` is one **canonical semantic Rust model**, and *both* operands are
> obtained by **projecting** an artifact into `M`.

Each artifact has a projection in and a materialization out:

```
            project (read)                       materialize (write)
spec text ───────────────►  M (canonical model)  ◄───────────────── spec text
document  ───────────────►  M                     ─────────────────► document
```

Then both directions are the *same* engine, differing only in which side is
"desired":

| Direction | desired = project(...) | current = project(...) | apply materializes into |
| --------- | ---------------------- | ---------------------- | ----------------------- |
| `compile` (→ doc)  | spec text | document  | the document |
| `dump` (→ spec)    | document  | spec text | the spec text |

The ECO is **always** `reconcile(desired, current)` over `M`. No text diff, no
byte diff — pure Rust-structure comparison. This is what makes "semantic ECO in
all directions" true by construction: there is one diff engine and one model.

---

## 2. Which Rust model is `M`? (the key open decision)

Three candidates exist in the tree today. The user is undecided; here is the
tradeoff so we can pick deliberately.

### Candidate A — the high-level API model (`altium-format/src/api/*`) ★ recommended

Types: `api::Component`, `Footprint`, `Pad`, `Pin`, `Net`, `Parameter`, … (per
domain). Designed (per `docs/high-level-api.md`) to **model every record type with
no passthrough**, domain types everywhere, natural keys.

- **Pro:** it *is* the document's native semantic model — the executor already
  writes through it (`add_component`, `add_footprint`). Lossless by design intent.
  Making it `M` means "the spec is literally a serialization of the high-level
  model" — exactly the user's framing.
- **Pro:** the dump direction (document → M) is then *free* — reading the document
  already yields this model.
- **Con:** lives in `altium-format`; the spec crate must project spec→api model
  (today the executor/compiler go spec→records more directly). Spec-only
  conveniences (library refs, templates, relative coords) must **resolve away**
  during projection — which is correct and desirable.
- **Implication:** Problem #1 (notes 02/05) restated cleanly: *make the spec
  projection and the document projection target the same complete `M`.* That is
  the schema-symmetry invariant, now concrete: one model, two projections.

### Candidate B — the `SpecModel` (`altium-format-spec/src/model.rs`)

Types: `SchLibSpec`, `ComponentSpec`, `FootprintSpec`, `PadSpec`, … Carries
`annotation` (identity) and authoring constructs.

- **Pro:** the reconciler/executor already use it; already has annotation IDs.
- **Con:** it is *spec-shaped* — to make it `M`, the **document** must project
  into a spec-shaped model, which re-creates R≠W (problem #1) from the other
  side. It also carries serialization-flavored constructs that shouldn't be in a
  canonical semantic model. Weaker fit for "spec is just a serialization."

### Candidate C — a dedicated normalized IR (grow `SyncSnapshot`, sync.rs)

Types today: `SyncSnapshot`, `SyncComponent`, `SyncPin`, `SyncNet` — a minimal
cross-document IR already used for spec↔spec sync.

- **Pro:** neutral, direction-agnostic by design; precedent exists.
- **Con:** currently minimal (components/pins/nets only); growing it to full
  fidelity duplicates the api model. Two models to keep in lockstep with the
  format → drift risk.

**Recommendation: Candidate A** — the high-level API model is `M`; `SpecModel`
becomes a (de)serialization layer that compiles **to/from** `M` (spec ⇄ M ⇄
document). This collapses three representations to one semantic core plus two
serializers, and makes the user's "spec is just the serialization format"
literally true. Identity/annotation metadata (note 01 `ResourceAddress`) attaches
*to* `M` entities (UniqueId/annotation-id alongside the natural key), since the
api model uses natural keys but we need stable identity for diffing across edits.

This is the decision to lock before building reconcile v2. Flagged in §8.

---

## 3. The low-level CFB diff is a separate, agent-facing artifact

The CFB diff (`CfbSemanticDiffReport` / `DiffIssue`, `test_utils.rs`) answers a
**different question for a different audience**:

- **Audience:** computer agents (including our own dev agents), not end users.
- **Question:** "does our serializer correctly support the file format?" — e.g.
  after implementing a record type, does apply write the bytes the format
  expects; does dump→apply→compare round-trip; did a refactor change the
  serialized output unexpectedly?

It is produced by serializing and comparing bytes — inherently a different plane
from the semantic ECO. It is **parallel, not nested**: `plan` can emit it
*alongside* the semantic ECO when asked (an agent/verify mode), but it is never a
drill-down beneath an `EntityChange`. (Dropping the earlier "ECO indexes the byte
diff" idea: users don't want byte offsets under their change order; agents want
the raw byte delta on its own terms.)

How it's produced for a *plan* (before anything is written): **dry-run apply,
then diff serializations.**

```
current document ── serialize ─► bytes_before ─┐
                                               ├─ diff_cfb_semantic ─► Vec<DiffIssue>
clone ── execute(ECO) ── serialize ─► bytes_after ─┘
```

Because it diffs the *serialization of the same ECO that apply executes*, it is a
faithful preview of the bytes apply will write — making it a precise format-
support verification signal. This is exactly today's `assert_cfb_files_semantic_eq`
/ `cfb diff --semantic` machinery, surfaced as an opt-in plan output.

Note the asymmetry vs §1: the semantic ECO is direction-agnostic (model-vs-model);
the CFB diff only applies where the **target artifact is a CFB document** (the
`compile` direction, and any doc→doc verification). For the `dump` direction the
target is spec text — and we **do not** diff that text (per the correction); the
semantic ECO already covers it. If we ever want "did dump write the spec bytes I
expected," that's a trivial separate string compare in tests, not part of this
architecture.

---

## 4. The Rust types

```rust
// ── the semantic change order (rename/extend today's eco.rs) ────────
pub enum Direction { Compile, Dump }

pub struct Plan {
    pub direction: Direction,
    pub target: PathBuf,                       // the artifact `apply` will write
    pub eco: EngineeringChangeOrder,           // semantic, on model M — ALWAYS present
    pub cfb_verify: Option<CfbSemanticDiffReport>, // agent/verify mode — opt-in, compile dir only
}

impl Plan {
    pub fn is_converged(&self) -> bool;        // exit 0 vs 2 (note 03 §2)
    pub fn render_eco(&self) -> String;        // user-facing ECO
    pub fn render_cfb_verify(&self) -> Option<String>; // agent-facing
    pub fn to_json(&self) -> Result<String>;   // both Serialize → saved plans (note 03 §3)
}
```

`EngineeringChangeOrder` already derives `Serialize`; add `Serialize` to
`DiffIssue` so the verify report is JSON-able for programmatic agent use.

### Generic driver over domains × directions: a `Provider` trait

One trait implemented per (domain, direction) so the generic `plan`/`apply` never
names a concrete type — this is the "one engine" of note 01 §2, and it guarantees
plan==apply because both call the same `reconcile`/`execute`:

```rust
pub trait Provider {
    type Model;                          // the canonical M for this domain
    fn project_desired(&self) -> Result<Self::Model>;          // from the source artifact
    fn project_current(&self, target: &Path) -> Result<Self::Model>; // from the target artifact
    fn reconcile(&self, desired: &Self::Model, current: &Self::Model)
        -> Result<EngineeringChangeOrder>;
    fn apply(&self, eco: &EngineeringChangeOrder, target: &Path) -> Result<()>;

    // only for compile-direction CFB verification (default: None)
    fn cfb_verify(&self, _eco: &EngineeringChangeOrder, _target: &Path)
        -> Result<Option<CfbSemanticDiffReport>> { Ok(None) }
}

fn plan<P: Provider>(p: &P, target: &Path, verify: bool) -> Result<Plan> {
    let eco = p.reconcile(&p.project_desired()?, &p.project_current(target)?)?;
    let cfb_verify = if verify { p.cfb_verify(&eco, target)? } else { None };
    Ok(Plan { eco, cfb_verify, /* … */ })
}
```

`reconcile` is one generic algorithm over `Model` (entity match by
`ResourceAddress`, field compare) — *not* re-implemented per domain. Per-domain
code is only the projection in/out and field enumeration. This is where today's
hand-kept reconciler/executor parity (note 00 §2) becomes structural.

---

## 5. Build on the existing low / high level APIs?

- **Semantic ECO → high-level API model (recommended `M`).** Build directly on
  `altium-format/src/api/*`. The executor already writes through it; the dump
  direction reads it for free. New work: a spec ⇄ `M` (de)serializer (refactor of
  today's compiler/executor and dump), and a *generic* `reconcile` over `M`
  replacing the hand-written per-type reconciler.
- **CFB verify → existing semantic CFB diff.** Reuse `CfbSemanticDiffReport` /
  `DiffIssue`. Small enabling additions: `to_bytes`/`open_from_bytes` on
  documents (today `save` only takes a path; `CfbDocument` is already
  `Cursor<Vec<u8>>`, so trivial), an in-memory `diff_cfb_semantic(&[u8], &[u8])`,
  and `Serialize` on `DiffIssue`.

**Privacy (CLAUDE.md):** the semantic ECO exposes only high-level model concepts
— clean. The CFB verify report exposes format internals (stream paths, block
indices, byte offsets) — but that's the same controlled exposure already granted
to `cfb diff --semantic`, and it's intentionally the agent/verify surface. Keep
it opt-in so default `plan` output is purely semantic.

---

## 6. Sequencing hooks (amends note 04)

- **Phase 1.1 `ResourceAddress`** becomes "identity metadata on `M`" — pick `M`
  first (§2), then attach identity. *Blocking decision.*
- **Phase 1.2 apply-via-ECO** = `Provider::apply` consumes the ECO; prerequisite
  for the dry-run CFB verify.
- **New foundation bits:** `to_bytes`/`open_from_bytes`, in-memory
  `diff_cfb_semantic`, `Serialize` on `DiffIssue`, `Plan`/`Direction` types.
- **Generic `reconcile` over `M`** replaces the per-type reconciler — sequence
  after `M` is chosen; this is the single biggest internal refactor.

---

## 7. Architecture options considered

- **One canonical model `M`, generic `reconcile`, two serializers (spec, doc).
  ✅ Recommended.** §1–§2. Direction-agnostic semantic ECO; "spec is just
  serialization" is literal; CFB verify stays a parallel agent artifact.
- **Keep `SpecModel` as the diff model (candidate B).** Re-creates R≠W from the
  document side; rejected as the canonical model (still fine as a *serializer*).
- **Per-direction bespoke diff engines.** Rejected — duplicates logic, drifts,
  and tempts text/byte diffs back in for the dump side (the thing we're removing).

---

## 8. Open questions

1. **Choose `M`** (§2). Recommendation: the high-level API model (candidate A),
   with `SpecModel` demoted to a serializer. This is the load-bearing decision;
   everything else composes once it's settled.
2. **Identity on `M`.** The api model uses natural keys; diffing across edits
   needs stable identity (UniqueId / annotation-id). How do these attach to `M`
   entities — a parallel `Identity` field, or a side-table keyed by natural key?
   (Ties to `ResourceAddress`, note 01 §3.)
3. **Generic `reconcile` field model.** To diff `M` generically we need each
   entity's comparable fields enumerable (derive macro? a `Diffable` trait? reuse
   the existing `OpsSchema`/derive infra?). Determines how much is generic vs
   per-type.
4. **CFB verify scope.** Confirm it is compile-direction-only and never offered
   for dump (per the correction), and that document-global byte changes (header
   timestamps, font tables) are reported plainly rather than confused with
   entity changes.
