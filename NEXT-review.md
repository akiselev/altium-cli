# NEXT.md Design Review

**Date**: 2026-06-21
**Review scope**: NEXT.md (Clean-Slate Bidirectional Plan/Apply Architecture) plus cross-reference against existing implementation and comparable architectures.

## Overall Assessment

The architecture is fundamentally sound. The three-way merge, plan-as-contract, identity
tiers, and layer separation are all correct choices. Version 0.1 already has a lossless CST and
merge engine implemented (`cst/`), so this isn't a greenfield proposal — it's a replacement
for the reconciler/executor/ECO machinery, which is currently ~11,725 lines of typed-but-flat
string-identity diffing code. The critique below focuses on what the design **omits or
underspecifies**, not what it gets wrong.

---

## Critical Gaps

### 1. The elaboration black box (Sections 4.2–4.3)

The largest hand-wave in the document. Between "authored intent model" and "concrete artifact
snapshot" sits a compilation step that resolves imports, templates, defaults, constraints, and
overrides into a complete Altium-shaped value. This IS the spec language's semantics. The
document describes it with one struct example (`ComponentOccurrenceIntent`) and the word
"compilation," but doesn't specify:

- **Override resolution order**: if a template defines `pad_hole: 0.8mm`, a component
  overrides `pad_hole: 1.0mm`, and an instance further overrides `pad_hole: 1.2mm`, what's
  the rule? Diamond imports (two templates both inheriting from a third) need explicit
  disambiguation or precise semantics.

- **Imports as separate resolution phase**: Dhall's model (resolve → type-check → normalize)
  is the right inspiration. Imports must be canonicalized by path, checked for cycles, and
  resolved to an import-free expression before any template/default/constraint resolution
  begins. Mixing these phases produces spooky action at a distance.

- **Constraint solving**: if `placement { }` declares `left_of $R1, $R2 { gap: 5mm }` and
  component footprints have different sizes, what does the resolver produce? Is it a concrete
  coordinate, a symbolic constraint left in the snapshot, or a solver invocation deferred to a
  later phase? The document never says.

- **What `Option::None` means in intent vs. snapshot**: Section 9 correctly identifies five
  distinct absence meanings, but the compiler needs to know, for each field, whether `None`
  means "inherit from template," "leave unchanged," or "explicitly not set." The current spec
  syntax conflates these (absence of a property could mean any of them), and NEXT.md doesn't
  specify how the compiler disambiguates them.

**The absence of formal elaboration semantics is the highest risk item in the design.** If
you can't write a specification for what `compile(authored_intent) → snapshot` produces on
representative inputs, the rest of the architecture (three-way plan, patches, apply) can't be
tested in isolation.

### 2. The ledger/external baseline is underspecified (Section 6.2)

Section 6.1 proves empirically that embedded metadata survives GUI save **only** as a hidden
component parameter (`RECORD=41`) and **only** at component scope. Therefore, the external
baseline file becomes the **primary** identity carrier for keyless primitives (all of PcbLib,
all PcbDoc primitives, SchLib PieGraphics), not a fallback. Despite this, the baseline format
is described with three structs and no schema:

- What is the concurrency model? If two users run `plan` simultaneously against the same
  file, does the baseline support locking or version-vector conflict detection?

- Is the baseline a single file, one per Altium file, or one per project?

- How is the baseline stored relative to the Altium file? Sidecar file (`.schlib-baseline`)?
  Project-level `.altium/` directory? User-global `~/.altium/`?

- What happens when the baseline is deleted? The document says "First brownfield adoption has
  no ledger...bootstrap binds by native id + natural key" — but doesn't say what file the
  bootstrapped baseline is written to or how it's discovered later.

The Pulumi state backend approach (pluggable, with local file as default and cloud as option)
is probably overkill, but the single-file model + versioning + locking should be specified
now.

### 3. The "compile" direction is underspecified compared to "dump"

The document is detailed about identity extraction from Altium files (Section 6.2 tiers) but
doesn't describe how identity is **assigned** in the compile direction. When `compile` creates
a new component in the Altium file, does it:

- Generate a new `UniqueId` (Altium-native)? Does `altium-format` support that?
- Assign a `BindingId` and write it as a hidden parameter?
- Defer identity assignment to the Altium file's own ID generation on save?

This matters because the next `plan` needs to match this entity. The design says "same
BindingId resolving on both sides...is an Update" — but someone has to mint the BindingId and
write it to both sides, and the document doesn't say who or when.

### 4. Missing: the spec syntax → authored intent mapping

Section 20 item 1 defers the decision of whether to retain, revise, or replace the existing
syntax. But the syntax IS the authored intent model's surface area. The two can't be designed
independently. For example:

- The current syntax has `let name = expr` bindings at block scope. How do these interact
  with templates and defaults? Is `let x = 1mm; pad { hole: x }` equivalent to
  `pad { hole: 1mm }`?

- The current syntax has `import "path"` which compiles to `SymbolRef::Import`. Does the new
  system resolve imports before or after intent model construction?

- Pin connections (`pin X -> #NET`) don't round-trip through dump. If the new system uses the
  same syntax, does the elastication layer need to understand pin connection shorthand and
  emit equivalent low-level wires?

The relationship between CST nodes and intent model entities also needs specification.
Currently, `#[annotation(id = "...")]` lives on the CST and survives edits. In the new
system, does `SourceId` replace annotations, supplement them, or is `SourceId` derived from
the CST node identity? The document uses both terms without clarifying the relationship.

### 5. Templates and generators are mentioned but not modeled

The authored intent model description says it "may contain templates and generators," but no
types, semantics, or syntax are specified. Templates are a major value proposition of a spec
language over raw Altium editing (define a footprint pattern once, instantiate with
parameters). If the first SchLib vertical slice doesn't include template semantics, the
elastication layer's design might be wrong for templates when they're added later.

### 6. Multi-file project synchronization is unaddressed

A PrjPcb references multiple SchDoc and PcbDoc files. When the user edits a SchDoc in Altium
GUI (adds a component), the change propagates to:

- The SchDoc file itself
- The PrjPcb's document list (if the SchDoc was added)
- The PcbDoc (via ECO synchronization in Altium)
- The spec files for all of the above

The document's three-way model is single-artifact. How does the planner handle cross-artifact
consistency? Is there a project-level plan that encompasses all files? If the user runs
`altium dump plan design.SchDoc`, does the plan show changes that would cascade to the PcbDoc?
The Pulumi model (where `ComponentResource` groups multiple `CustomResource`s) is relevant,
but the document doesn't address project scope.

### 7. Plan versioning and compatibility

The plan format (Section 10) must be the stable API contract between `plan` and `apply`.
Missing specification:

- What serialization format (JSON, msgpack, protobuf, custom binary)?
- Version compatibility policy: can a plan generated by v1.2 be applied by v1.3? Can v1.3
  plans be applied by v1.2? Is there a minimum version gate?
- What happens when the schema version in a saved plan is unrecognized? Hard error? Migration
  path?
- How are plan files named/discovered? Is the plan filename meaningful or is it an opaque
  artifact keyed only by content hash?

Terraform manages this with a version byte in the binary plan header and explicit version
gates: old plans can be applied by new binaries (forward compat), new plans are rejected by
old binaries. The document should specify the same.

---

## Design Choices That Need Justification

### The `PatchOp<T>` model (Section 10.2)

The design prefers `Create`/`Replace`/`Delete` with whole-value replacement rather than
field-by-field property setters. This is correct for structural integrity (avoids
intermediate states that violate invariants) but has a subtle consequence: when a component
has 20 properties and the user changes one in the spec, the patch replaces the **entire**
component through `update_component`. This means:

- The executor must read the existing component, apply the patch, and write the whole thing
  back. The existing executor (`executor.rs`) already does this for most entity types.
- The patch becomes large (serializing an entire component for a one-field change).
- The precondition hash covers the entire component, so independent changes to different
  fields on the same component from different authors will always conflict.

This is the right tradeoff (correctness over granularity), but should be documented
explicitly because it means the system can never support concurrent independent field edits
to the same entity by different authors — the plan will always report a Conflict.

### Tier 3 identity: ordinal + fingerprint (Section 6.2)

Using `(parent, collection, ordinal, fingerprint)` for keyless primitives means:

- **Identity breaks on reordering**: if Altium re-serializes PcbDoc primitives in a different
  order (which it can do on any GUI save), all affected primitives become "new" entities with
  renumbered ordinals. The fingerprint matching (step 5) catches some of these, but multiple
  identical primitives (e.g., several 10mil tracks on the same layer) collide.

- **Fingerprint excludes management metadata** (correct), but this means the system can't
  detect when a primitive was unchanged except for its embedded BindingId being written (it
  shouldn't be, since Tier 3 is ledger-only, but it's worth stating).

- **Delete+add+review** for ambiguous cases is the right conservative approach, but the
  user-facing UX for reviewing "this track was deleted and re-added because its ordinal
  shifted from 142 to 147" needs design. Showing raw ordinals to users is useless.

### The five managed-scope absence meanings (Section 9)

The distinction is correct but the implementation implications are underspecified:

- **Inherit from template** and **Leave unchanged** look identical at the spec syntax level
  (property not written). The compiler needs a way to distinguish them — either a marker
  syntax (`key: inherit` or `key: _`), or a rule that "within a template block, absent means
  inherit; within an instance block, absent means leave unchanged."

- **Reset to format default** and **Clear/delete the value** are distinguished only when the
  property has a meaningful default. For properties where the Altium format has no default
  (e.g., many PCB primitive properties default to zero), these are equivalent in practice
  but distinct in the model.

---

## Reference: Architecture Patterns from Other Systems

### Three-way merge (Kubernetes SMP + Helm 3)

The `Base → Source, Base → Document` diff pattern is exactly right. Kubernetes' three-way
merge formula is the reference: if a field is unchanged between base and new-config, preserve
the live value (it was changed by another actor). If changed, apply the new value. The
document's `ChangeDisposition` enum captures this correctly.

However, Kubernetes' experience also shows that **array/list merging is the hard case**.
Strategic Merge Patch uses `patchMergeKey` to match list elements by a key field rather than
position. The document's identity tiers address this for components (matched by
`lib_reference`) and pads (matched by `designator`), but for **keyless ordered collections**
(PcbDoc tracks, arcs, fills), position IS the identity. Kubernetes' answer is to use
`$patch: replace` (replace the whole list) when position matters, or `$setElementOrder` to
reorder. The document's Tier 3 ("ordinal + fingerprint") is essentially position-as-identity
with a fingerprint guard, but it will produce false delete+add events when primitives are
reordered during GUI editing (Altium re-serializes collections in arbitrary order). This is a
real limitation that should be acknowledged more explicitly.

### Lens theory: complements and alignment

The symmetric lens model (Hofmann/Pierce/Wagner) formalizes exactly what NEXT.md's baseline
does: a **complement** stores information that exists in only one side, and **alignment**
tracks correspondences. The key insight from edit lenses is that alignment is a **dynamic
structure** that both `putr` and `putl` must update. Applied here: when the planner detects
that a component was renamed in the Altium document, the alignment (ledger entry) must be
updated to reflect the new `lib_reference`, but the `BindingId` stays the same. This is what
Section 6.2 says ("renames are free" for Tier 1), but it doesn't specify that the ledger
update is itself a plan output.

More importantly, edit lens theory demands that every primitive (lens) satisfy **roundtrip
laws**. The document's Section 17 laws are correct but incomplete — they're stated for the
top-level `apply ∘ plan ∘ snapshot` composition, not for the individual compilation/projection
layers. Each layer should satisfy its own roundtrip law:

- `project ∘ materialize == id` (document → snapshot → document produces the same document)
- `elaborate ∘ compile == id` (for brownfield inline content, intent → snapshot → intent
  produces equivalent intent)

These are harder to define for the intent model (what does "equivalent intent" mean?), which
is why they're absent. But the projection law (`project ∘ materialize == id`) should be
specified and testable for the Altium document → snapshot → document roundtrip.

### Dhall: import resolution as a separate phase

Dhall's architecture (resolve imports → type-check → normalize → marshal) is the right
pipeline model for the spec language. Applied to NEXT.md:

```
spec text → CST → resolve imports (produces import-free CST)
                  → resolve templates/defaults (produces instance CST)
                  → compile to authored intent model
                  → elaborate to concrete artifact snapshot
```

The document compresses these into "compile" but the phases have different failure modes and
testing requirements. Separating them lets you test import resolution independently of
template expansion.

### Rowan: red/green trees for CSTs

The existing `cst/` module uses `cstree` (a Rowan-like crate). This is the correct choice.
The key operational detail: structured CST edits should use a `replace_node` pattern that
walks the green tree, replaces specific green nodes, and produces a new green root with
`Arc`-shared unchanged subtrees. The existing `cst/edit.rs` already does text-splicing by
byte offset, but a tree-based approach would be more robust for complex edits (inserting
blocks at specific positions, reordering siblings).

### Terraform: plan file as API contract

Terraform's saved plan format is a zipped protobuf containing the diff + prior state. The
document's `PlanBundle` should follow this pattern exactly: serialize prior state (baseline),
desired changes (patches), and preconditions (hashes) together. The plan file IS the API
between `plan` and `apply` — it must be self-contained (no references to external state that
could change).

What the document gets right that Terraform doesn't: bidirectional changes (a single plan can
update both the spec and the Altium document), and the plan is the same format for both
`compile` and `dump` directions.

### Pulumi: logical name + URN + physical ID

Pulumi's resource identity model (logical name unique in program, URN = fully qualified path,
physical ID assigned by provider) maps directly to the document's `BindingId` (stable handle)
+ `ResourceAddress` (runtime locator). The key lesson: decouple the stable identity from the
address, because address changes (component renamed) should not be treated as
delete+create.

### Kubernetes Server-Side Apply: field ownership

SSA's field manager model (each field tracked as owned by a specific "manager") maps to NEXT.md's
`Authority` enum. The specific lesson: when Authority=Spec owns a field and Authority=Altium
edits the same field, this is a Conflict. When only one side owns the field, the other side's
changes to it are silently accepted. The document's `ManagedScope` struct is essentially an
SSA field manager, but it operates at entity granularity rather than field granularity. Field-
level ownership may be needed later (e.g., spec owns component value, Altium owns component
position), and the model should accommodate this.

---

## Minor Issues

- **Section 20 item 10**: "Which current spec-language features belong in the first SchLib
  vertical slice" — this is the right question but it needs an answer. The current syntax
  supports `component`, `pin`, `footprint` (optional), `part`, `alias`, `parameter`,
  `swap_group`, `graphic_type { ... }`. Which subset ships first? Recommendation: component
  with pins, parameters, and properties only. Footprints, parts, aliases, and graphics are
  second-pass.

- **Section 4.4 (design graph)**: The document correctly defers the vendor-neutral graph, but
  the "two models and a projection" of Option C becomes "three models and two projections"
  of Option D. The design should anticipate this by defining snapshot types as **traits** so
  the future graph can implement them without changing the planner.

- **Section 14 (crate boundaries)**: `altium-reconcile` is a misleading name. It does plan,
  apply, syncing, identity, and ECO rendering. `altium-sync` or `altium-workflow` would be
  better.

- **Missing: error recovery in plan application**: Section 12 steps 9-10 (commit via renames
  + journal) is correct for filesystem operations, but what about partial application? If
  three Altium files and two spec files need updating, and the fourth file write fails, the
  rollback strategy is underspecified. The journal must track enough state to undo partial
  commits.

- **Section 17 laws**: The stated laws are correct but use undefined operations (`project`,
  `materialize`). Each should be formally defined:

  | Law | Definition |
  | --- | --- |
  | `project(document) == snapshot` | The high-level API → snapshot projection must be deterministic and lossless for all supported fields |
  | `materialize(snapshot) then project == snapshot` | Writing a snapshot to a document and reading it back must produce an equivalent snapshot |
  | `apply(plan(base, source, document)) == planned after-state` | Execute the plan's patches on the current artifacts; result matches what the plan described |
  | `replanning after apply produces no changes` | A second plan with the new baseline and unchanged artifacts produces an empty plan |
  | `applying a stale plan fails` | If artifact hashes don't match plan preconditions, refuse to apply |
  | `independent same-resource edits produce Conflict` | Two authors changing different fields of the same entity detected as conflict (given the whole-value-replace patch model) |
  | `unsupported or unrepresentable content blocks planning` | Any Altium data we can't model in snapshots is a hard plan error, not silently preserved |

---

## Architecture Options Re-evaluation

NEXT.md §15 evaluates four options and recommends Option C (authored intent + concrete Altium
snapshot). This is the right choice, but the document understates the complexity:

| Aspect | NEXT.md says | Reality |
| --- | --- | --- |
| "Two models and a required elaboration/projection layer" | Listed as a con | This IS the system. The elaboration layer is the entire novel contribution; the three-way merge is textbook. |
| "Must prove that compiling intent and importing documents converge to equivalent snapshots" | Passed over quickly | This proof requires formalizing what "equivalent snapshots" means (semantic equivalence modulo vendor-format artifacts) and is non-trivial for any non-trivial spec. |
| "More initial design work than Option A" | Listed as a con | The design work isn't in the snapshot types (those are straightforward projections of the Altium API). It's in the **elaboration semantics**: imports, templates, defaults, overrides, constraint resolution. The document defers this work to the "semantic compilation" step without designing it. |

The recommendation for Option C is correct, but the document should be honest that the
elastication layer is the hard problem, not a sidebar. A separate design document focused on
the spec language semantics (separate from the plan/apply machinery) is justified.

---

## Verdict

The design is architecturally mature and correctly identifies the hard problems
(heterogeneous identity, keyless primitives, managed scope semantics, plan immutability). Its
main weakness is treating the **elastication/compilation layer** as a solved sub-problem when
it's actually the core contribution. The three-way merge and plan/apply machinery are
well-understood patterns from Kubernetes and Terraform. The language semantics (imports,
templates, defaults, overrides, constraints → concrete snapshot) are novel and need their own
design document with formal semantics before implementation.

### Priority of unresolved decisions (from §20)

1. **(Highest) Elaboration semantics**: How does the compiler resolve imports, templates,
   defaults, overrides, and constraints into a concrete snapshot? This must be designed before
   the SchLib vertical slice, because it defines what the authored intent model IS.
2. **(High) Spec syntax**: Retain, revise, or replace? Tied to item 1 — the syntax is the
   surface area of the elastication layer.
3. **(High) External baseline format**: File naming, versioning, concurrency, per-file vs
   per-project. Required before any plan/apply can be written (identity depends on it).
4. **(Medium) CST technology**: The existing `cstree`-based CST is already implemented. The
   only open question is migrating from byte-offset text splicing to tree-based structured
   edits.
5. **(Medium) Plan serialization format**: JSON, msgpack, or protobuf? Must be decided before
   implementing plan save/load.
6. **(Lower) Conflict resolution UX**: Can be deferred — the first version can report
   conflicts as blocking errors with no resolution mechanism.
7. **(Lower) Embedded metadata for PcbLib/SchDoc/PcbDoc/PrjPcb**: Can be validated
   incrementally; only SchLib is needed for the first vertical slice.
