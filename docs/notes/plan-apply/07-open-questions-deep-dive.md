# Plan/Apply Redesign — Note 07: The four open questions, in depth

**Session date:** 2026-06-18

Deep dive on note 06 §8's four open questions. They are not independent — Q1
(which model) largely determines Q2 (identity) and Q3 (generic diff), and Q1+Q3
together decide whether "partial specs" survive. The cross-coupling is in §5.

Evidence gathered from the code, referenced throughout:
- `api::Component`/`Pin`/`Parameter`/`Pad` (`altium-format/src/api/*_types.rs`)
- `SpecModel`/`PadSpec` (`altium-format-spec/src/model.rs`)
- `reconciler.rs::diff_pcb_pads` (the hand-written diff)
- `OpsSchema` derive (`altium-format-derive/src/lib.rs:575`) — defined, **unused**
- STATUS.md "Roundtrip Known Differences (Acceptable)"

---

## Q1 — Which Rust model is the canonical `M`?

### What the question really is

`reconcile` diffs two values of one type `M`, and both the spec and the document
project into `M` (note 06 §1). So `M` is *the* schema of the whole system: the
thing the ECO is expressed over, the thing apply materializes, the thing dump
reads into. Picking it wrong taxes every other decision.

### Why it matters — concrete evidence

The candidates are not equally expressive *today*:

- **`api::Pin`** has `show_name`, `show_designator`, `unique_id`, `color`,
  `symbol_inner_edge`, … — 30+ fully-typed fields. It models the record.
- **The SpecModel** *drops* several of these on purpose — STATUS.md §10 lists pin
  `show_name`/`show_designator`, graphic colors/widths, pad `corner_radius_pct`,
  etc. as "silently normalized on apply." The spec is a **lossy** view.

So `api` model ⊋ SpecModel in fidelity. That asymmetry **is** Problem #1 (notes
02/05) in miniature: dump (reads the rich `api` model) can emit detail apply (via
the lossy SpecModel) can't consume.

### Options & tradeoffs

| | A. high-level `api` model | B. `SpecModel` | C. new IR (grow `SyncSnapshot`) |
|---|---|---|---|
| Exists today | ✅ full fidelity, models every record | ✅ but lossy (§10) | partial (components/pins/nets only) |
| "Spec is just a serialization" | ✅ literal: SpecModel ⇄ `api` ⇄ doc | ✗ spec *is* the model | ✅ but two serializers to write |
| Dump direction (doc→M) | ✅ free — reading the doc yields M | ✗ must down-project to lossy model (re-creates Problem #1) | ✗ must write doc→IR projection |
| Identity (Q2) | ✅ `unique_id` already on fields | ✅ `annotation` already on blocks | ✗ must add |
| Lives in | `altium-format` | `altium-format-spec` | either |
| New work | spec ⇄ M serializer; generic reconcile | least (status quo) | most (full new model + 2 projections) |
| Risk | M leaks `api` types into spec crate's diff (already a dependency) | institutionalizes lossy diff | third model drifts from format |

### Recommendation

**A — the high-level `api` model is `M`; `SpecModel` is demoted to a serializer**
(spec text ⇄ `M`). This makes the user's "the spec is just the serialization
format" literally true, makes the dump direction nearly free, and gives identity
for free (Q2). It also forces Problem #1 to be solved *as a precondition* rather
than worked around: you cannot have a lossy `SpecModel` serializer if round-trip
must hold, so the §10 normalizations become serializer gaps to close, tracked and
fail-loud (note 02 §4), not silent.

The cost is the real one: a generic `reconcile` over `api` types (Q3) and a
spec⇄`api` serializer replacing today's compiler/executor/dump. That is the
biggest internal refactor in the whole plan — but it is the refactor that retires
the entire class of hand-maintained drift bugs.

---

## Q2 — How does stable identity attach to `M`?

### What the question really is

To diff two `M`s you must first decide *which entity in A corresponds to which in
B*. The `api` model identifies by **natural key** (`lib_reference`, pin
`designator`, `pad_name`). Natural keys break under exactly the edits we care
about:

- **Rename:** change a designator → natural-key match sees delete-old + add-new,
  losing the "this is the same pin, renamed" intent (and any reconciliation of
  its other fields).
- **Nameless primitives:** PcbDoc tracks/fills have no natural key at all — the
  documented PcbDoc "every primitive reports as ADD" failure (spec-problems §7).

So `M` needs a **stable identity** distinct from its natural key. The `ResourceAddress`
of note 01 §3 is that identity; Q2 is *where it physically lives*.

### Why it matters — concrete evidence

Good news: the anchors largely already exist. `api::Pin.unique_id`,
`api::Parameter.unique_id`, `api::Graphic` (matched by `unique_id` in
`reconciler.rs:948` today). The SpecModel side already mints `annotation` short
IDs. So the binding "annotation-id ↔ UniqueId" (note 05 §2, the bidirectional
two-way link) is *representable now*; it just isn't used uniformly.

### Options & tradeoffs

1. **Identity as a field on each `M` entity** (e.g. every entity has
   `identity: ResourceAddress`).
   - *Pro:* self-contained, travels with the entity, simple to diff.
   - *Con:* must thread it through every `api` type and every projection;
     nameless primitives need a *derived* (content-hash) identity computed at
     projection time, so the field is "sometimes intrinsic, sometimes derived."

2. **Identity as a side-table** keyed by natural key / position
   (`HashMap<NaturalKey, ResourceAddress>` built during projection).
   - *Pro:* `api` types stay unchanged; keeps derived identity out of the model.
   - *Con:* a second structure to keep in sync; lookups everywhere; awkward for
     nameless primitives whose "natural key" is itself the content hash.

3. **A layered identity resolver** — a function
   `fn identity(entity: &E, ctx) -> ResourceAddress` with the priority ladder from
   note 01 §3 (annotation-id → unique-id → natural key → structural/content hash).
   The diff calls it; nothing is stored on `M`.
   - *Pro:* one place encodes the whole priority policy; works for both intrinsic
     and derived identities; `M` stays clean.
   - *Con:* identity is recomputed each diff (cheap); the resolver must be
     domain-aware.

### Recommendation

**Option 3 (resolver) + intrinsic anchors where they exist.** Store the
`unique_id` already on the entity (don't remove what's there), but compute the
*effective address* through one resolver that implements the priority ladder. This
keeps the model honest (no synthetic identity fields polluting it), handles the
nameless-primitive content-hash case uniformly, and puts the entire rename/match
policy in one auditable place. The structural-hash decision for nameless
primitives (which fields are "identity" vs "attribute") is its own sub-question —
it's note 04 open-question #3 and needs per-primitive review against the format
docs, because a wrong split causes spurious Replace churn.

---

## Q3 — How does generic `reconcile` enumerate comparable fields?

### What the question really is

Today `reconcile` is **hand-written per type**. `diff_pcb_pads` is ~150 lines of:

```rust
if spec_pad.at != existing_pad.location { prop_changes.push(PropChange{ field:"location", ... }); }
if let Some(shape) = spec_pad.shape { if shape != existing_pad.shape { ... } }
if let Some(x_size) = spec_pad.x_size { if x_size != existing_pad.x_size { ... } }
// … one block per field, each formatting old/new by hand …
```

This is the direct cause of the fix-#19 bug class (spec-problems §4 #19): a field
applied by the executor but **forgotten** in this hand-written list → plan says
"unchanged," apply mutates. Every new field is two edits that must agree. Q3 asks:
can the field enumeration be generated instead of hand-written?

### Why it matters — concrete evidence

- The drift bug is *structural* to hand-writing (note 00 §2). Generic diff kills
  it: there is nothing to forget.
- Infrastructure for this **already exists but is unused**: the `OpsSchema` derive
  (`altium-format-derive/src/lib.rs:575`) generates a static `FieldSchema {
  rust_field, name, ty, required }` table per struct. It was clearly built for a
  reflection-style "ops" system (`crate::ops::schema`) that is not wired up. It
  gives field *names/types* but **not values** — so by itself it can't compare; it
  proves the team already reached for reflection here.

### Options & tradeoffs

1. **Status quo: hand-written per-type diff.**
   - *Pro:* zero new machinery; full control of formatting/semantics per field.
   - *Con:* the drift bug is permanent; O(fields) maintenance forever; the
     biggest source of plan≠apply.

2. **A `Diffable` derive macro** generating
   `fn diff(&self, other:&Self) -> Vec<FieldChange>` — the macro emits the
   per-field comparison once, at the type, using each field's `PartialEq` +
   `Display`/`Debug`.
   - *Pro:* type-safe, no runtime reflection, handles each field's own formatting;
     adding a field auto-includes it in the diff (drift impossible). Mirrors the
     existing `FromParams`/`ToParams` derive philosophy.
   - *Con:* a real proc-macro to write; needs attributes for opt-outs
     (`#[diff(skip)]` for derived/cache fields), custom formatters (Coord as
     mils), and identity-vs-attribute marking (ties to Q2). Nested
     structs/collections (pins within a component) need recursion rules.

3. **Runtime reflection to a dynamic field map**
   (`fn fields(&self) -> BTreeMap<&str, FieldValue>`, diff maps generically).
   - *Pro:* one generic diff function for all types; could reuse/extend the
     `OpsSchema` direction.
   - *Con:* needs a `FieldValue` enum that can hold every field type (loses static
     typing); formatting/units handled centrally and bluntly; harder to give good
     per-field rendering; effectively reinvents serialization.

4. **Diff via an existing serialization** — e.g. both sides → param-map
   (`ToParams`) and diff keys (this is literally what the CFB *semantic* diff does
   for text blocks).
   - *Pro:* reuses `ToParams`; for schematic (param-based) records it's natural
     and already proven.
   - *Con:* PCB records are binary, not param-maps — no uniform serialization to
     diff; and diffing serialized strings is exactly the "don't diff the
     serialization, diff the model" thing the user rejected for the spec. Mixing
     planes.

### Recommendation

**Option 2 — a `Diffable` derive.** It is the same proven pattern as the crate's
existing derives, makes drift structurally impossible, and keeps diffs type-safe
and per-field-formattable. Pair it with attributes that double as the Q2 hooks:
`#[diff(id)]` marks identity fields (feeds the resolver), `#[diff(skip)]` excludes
derived/cache fields, `#[diff(with = fmt_mils)]` customizes rendering. The unused
`OpsSchema` infra is evidence this is the intended direction; `Diffable` is its
value-aware sibling. Reserve option 4 (param-map diff) only as the *low-level CFB
verify* mechanism (Q4), where serialization-plane diffing is the actual point.

---

## Q4 — Scope and noise-floor of the CFB verify diff

### What the question really is

The CFB verify diff (note 06 §3) compares *serialized bytes* to check format
support. Two scoping questions: (a) is it compile-direction-only? and (b) how does
it avoid reporting **format-normalization noise** as if it were real change?

### Why it matters — concrete evidence

STATUS.md "Roundtrip Known Differences (Acceptable)" enumerates changes that
happen on *every* save and are **not** semantic:

- All types: font-name buffer zero-fill (vs Altium heap garbage); boolean
  normalization (non-zero → 0x01).
- PcbLib: text WideStrings upgrade; via format upgrade (ext_size 42→45);
  SharedUnion NUL terminator.
- PcbDoc: pad sub4 upgrade (171→172 bytes); via section 4/5 always written;
  Rules6 tier2 serialization; param key ordering; duplicate param dedup.

Plus genuinely document-global bytes: header timestamps, font tables. A naive
byte diff drowns the real signal in these. The CFB *semantic* diff already
tolerates some (duplicate param pairs, zlib levels — CLAUDE.md), which is exactly
the right instinct, but the allowlist must be explicit and maintained.

### Options & tradeoffs

- **(a) Direction scope.** Recommendation: **compile-direction-only.** The dump
  target is spec text; per the user's correction we do *not* diff spec text — the
  semantic ECO covers the dump direction entirely. A "did dump write the bytes I
  expected" check is a trivial string compare in tests, not part of this
  architecture. So `Provider::cfb_verify` returns `None` for dump providers (note
  06 §4 default).

- **(b) Noise floor.** Three sub-options:
  1. **Raw byte diff** — rejected: buried in normalization noise, unusable as a
     verification signal.
  2. **Reuse the existing semantic CFB diff** (`CfbSemanticDiffReport`) with its
     param-order/zlib tolerances — the current baseline. Good, but its tolerances
     are partly hard-coded.
  3. **Semantic CFB diff + an explicit, versioned "acceptable upgrade" allowlist**
     derived from the STATUS.md list (e.g. `IgnoreViaExtSizeUpgrade`,
     `IgnorePadSub4Upgrade`). Each entry documents *why* it's noise and links to
     the format upgrade it represents.
  - *Tradeoff:* an allowlist can hide a real regression if too broad. Mitigation:
    allowlist entries are **typed and narrow** (specific stream/record/upgrade),
    never "ignore this stream," and the verify report counts how many issues were
    suppressed so an agent sees "0 real, 3 known-upgrade" rather than silence.

### Recommendation

Compile-direction-only; build on the existing semantic CFB diff; add a **narrow,
typed, documented allowlist** for the STATUS.md known upgrades, and always report
suppressed-count so suppression is visible, never silent (consistent with
fail-fast). Document-global changes (timestamps, font tables) go in a clearly
labeled "document-level / not entity-attributable" bucket so an agent never
confuses them with primitive changes.

---

## 5. How the four interact (read this before deciding any one)

The questions form a dependency chain, and one choice cascades:

```
Q1 (M = api model)  ──►  Q2 identity is mostly free (unique_id already on M)
        │                Q3 diffs api types  ──►  Diffable derive on api types
        │
        └──►  "partial spec" semantics must change (see below)
Q4 is independent of Q1–Q3 (different plane), but only exists if a CFB target does.
```

### The sleeper interaction: partial specs die when `M` goes concrete

Today, "the spec only mentions some fields" is encoded two ways at once:
- `SpecModel` fields are `Option<T>` (`PadSpec.shape: Option<…>`), and
- the reconciler guards every compare with `if let Some(x) = spec.field`.

That is the *partial-spec* world: unmentioned fields are left alone on apply.

If `M` is the concrete `api` model (Q1=A), there is **no per-field `Option`** —
`api::Pad.shape` is a concrete `PadShape`. So the partial-spec bit disappears from
the model. You then must choose how "I didn't mention shape" is represented:

- **(i) Complete specs (authoritative).** The spec serializer always emits every
  field; a spec *is* a complete `M`. Anything absent = default/delete. This aligns
  with note 03 §4 (authoritative delete), note 05 (dump emits complete specs), and
  "spec is just a serialization of M." **Simplest, most consistent — recommended.**
- **(ii) Keep presence info** via a parallel mask or `Option` wrapper *at the
  serializer boundary only* (not in `M`), so a partial hand-written spec means
  "merge onto current." More flexible for hand-authoring, but reintroduces
  partial-merge semantics and the "can't reset to default" ambiguity.

This is why Q1 is load-bearing beyond just "where do types live": choosing the
`api` model nudges the whole system toward **complete, authoritative specs +
Delete + Diffable**, which mutually reinforce. Choosing `SpecModel` (B) keeps the
**partial-merge + Option-fields + hand-written diff** world. They are two coherent
designs; mixing them is where bugs live.

### Bottom-line recommended bundle

1. **Q1: `M` = high-level `api` model**, `SpecModel` → serializer.
2. **Q2: identity resolver** (priority ladder) using the `unique_id` anchors
   already on `M`.
3. **Q3: `Diffable` derive** on `api` types, with `#[diff(id/skip/with)]`
   attributes that also feed Q2.
4. **Q4: compile-only CFB verify** on the existing semantic diff + a narrow typed
   allowlist with visible suppressed-counts.
5. **Consequence (accept deliberately): complete authoritative specs**, not
   partial-merge (§5(i)).

Each is individually defensible; together they're one coherent system where
plan==apply is structural, the spec is genuinely just a serialization of `M`, and
the CFB verify stays a separate agent-facing plane.
