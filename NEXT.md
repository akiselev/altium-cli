# Clean-Slate Bidirectional Plan/Apply Architecture

Status: proposed plan of record for the next implementation cycle.

Implementation checkpoint (2026-06-21): the lossless structured CST, typed
accessor/edit layer, and comment-preserving `dump` update path described in
sections 4.1 and 11 are implemented. The old CLI `spec_merge` text splice has
been removed. The baseline, three-way planner, saved-plan transaction, and
document patch portions of this plan remain future work.

This document captures the clean-slate design discussion for replacing the existing spec-language
dump/reconciler/executor/apply machinery. Future work should start here rather than extending the
current reconciler or executor.

## 1. Decision boundary

Preserve:

- `altium-format` low-level parsing and serialization implementation.
- `altium-format` high-level typed document APIs.
- Domain types and constants from `altium-format-types`.
- The repository-wide fail-fast and no-opaque-data rules.

Treat as replaceable:

- The current spec `dump.rs` implementation.
- The current `reconciler.rs` implementation.
- The current `executor.rs` implementation.
- The current report-oriented `eco.rs` model.
- `altium-cli/src/spec_merge.rs`.
- The current root `dump`, `plan`, and `apply` orchestration.
- The current `SpecModel` shape and compiler architecture where they obstruct the new design.
- Existing spec syntax implementation details. Preserve useful language behavior deliberately, not
  accidentally.

Do not start by incrementally teaching the current `EntityChange` report to execute. Its string
identities and string-valued properties are not a safe executable protocol.

## 2. Core conclusion

This is not fundamentally a two-way Terraform reconciliation problem. Both artifacts are editable,
so safe synchronization needs three inputs:

```text
                 last synchronized state
                           Base
                          /    \
                         /      \
              authored spec    Altium document
                 Source             Document
```

The new system is a three-way bidirectional synchronization engine:

```text
Spec text
   |
   v
Lossless CST --> Authored Intent Model --resolve/import/default-->+
                                                               |
Altium file --high-level API--> Concrete Artifact Snapshot -----+
                                                               v
Sync baseline ------------------------------------------> Three-Way Planner
                                                               |
                                                               v
                                                          PlanBundle
                                             +-----------------+-----------------+
                                             v                                   v
                                      SpecSourcePatch                    AltiumDocumentPatch
                                             |                                   |
                                             +---- stage -> validate -> commit --+
```

The defining invariant is:

> `apply` executes the exact typed patches produced by `plan`.

The semantic ECO is a rendered view of that plan. It is not itself the mutation protocol.

## 3. Preserved Altium API boundary

`altium-format` remains responsible for understanding Altium formats:

- CFB and stream layout.
- Records, primitives, sidecars, and ownership reconstruction.
- Strict typed parsing and serialization.
- Format-version upgrades.
- High-level document types and mutation methods.
- Document invariant validation.

The new workflow code must not reach into private records, streams, parameter collections, or raw
binary structures. If the high-level API cannot express a required operation, expand the high-level
API first with fully typed support.

Low-level details stay private to `altium-format`. The workflow layer operates only on high-level
API objects and its own authored/synchronization models.

## 4. Model layers

Do not force authored intent, resolved document state, and persisted source syntax into one model.
They have different semantics.

### 4.1 Lossless source CST

The spec parser must produce a lossless concrete syntax tree that preserves:

- Comments.
- Whitespace and formatting.
- Block and property ordering.
- Imports, templates, bindings, and expressions.
- Explicit versus omitted properties.
- Source spans and stable syntax-node identities.

Unknown syntax is a parse error. It is never silently retained as an opaque node.

The CST is the target of `dump apply`. Existing specs are updated with structured CST edits, not
regenerated wholesale and not merged by top-level text replacement.

### 4.2 Authored intent model

The authored model represents what the source means before it is expanded into an Altium document.
It may contain:

- Imports and reusable definitions.
- Definition/occurrence relationships.
- Templates and generators.
- Defaults and inherited values.
- Instance overrides.
- Placement and routing constraints.
- Explicit unset/reset operations.
- Source bindings and annotations.

Conceptually:

```rust
struct ComponentOccurrenceIntent {
    id: SourceId,
    designator: String,
    definition: DefinitionRef,
    placement: Option<PlacementIntent>,
    overrides: Vec<OverrideIntent>,
}
```

The authored model is not compared directly with Altium API objects.

### 4.3 Concrete artifact snapshot

Compilation resolves imports, templates, defaults, constraints, and overrides into a complete,
Altium-shaped semantic snapshot:

```rust
enum ArtifactSnapshot {
    SchLib(SchLibSnapshot),
    PcbLib(PcbLibSnapshot),
    SchDoc(SchDocSnapshot),
    PcbDoc(PcbDocSnapshot),
    PrjPcb(PrjPcbSnapshot),
}
```

The document side projects through the high-level APIs into the same snapshot family.

Reuse high-level API types inside snapshots where they are complete and have suitable ownership
semantics. Wrap or supplement them where synchronization needs additional information such as:

- Stable binding identity.
- Ownership and occurrence paths.
- Managed/unmanaged coverage.
- Source provenance.
- Collection ordering.
- Semantic fingerprints.

The existing high-level APIs are the Altium artifact boundary, not necessarily the long-term
vendor-neutral canonical design graph.

### 4.4 Optional future design graph

Do not make a new vendor-neutral design graph a prerequisite for reliable plan/apply.

Near-term flow:

```text
Authored Intent -> Concrete Altium Snapshot
```

Possible future flow:

```text
Authored Intent -> Vendor-Neutral Design Graph -> Concrete Altium Snapshot
```

Any future graph must still obey the no-opaque-data rule. The existing draft design-graph documents
that permit opaque preservation cannot be adopted unchanged.

## 5. Authority and metadata are orthogonal

Do not infer authority from whether a component import resolves.

```rust
enum Authority {
    Spec,
    Altium,
}

enum MetadataPolicy {
    NativeOnly,
    ManagedEmbedded,
    ManagedExternal,
}
```

Supported workflows:

| Workflow | Authority | Metadata policy | Meaning |
| --- | --- | --- | --- |
| Greenfield managed | Spec | ManagedEmbedded or ManagedExternal | Spec owns design intent; Altium is generated but GUI edits can be folded back. |
| Brownfield managed | Altium | ManagedEmbedded or ManagedExternal | Existing Altium design remains authoritative, but durable bindings and baselines are allowed. |
| Brownfield non-invasive | Altium | NativeOnly | Never add tool metadata; use native IDs and explicit/heuristic matching only. |

Import/template resolution determines materialization behavior, not workflow authority.

## 6. Identity and bindings

Keep distinct identity roles:

```rust
struct BindingId(...);        // common identity across source and Altium artifacts
struct SourceId(...);         // identity of an authored source entity
struct AltiumNativeId(...);   // Altium UniqueId or equivalent
struct ResourceAddress(...);  // scoped runtime address used by plans
```

Matching priority:

1. Existing managed `BindingId` shared across artifacts.
2. Native Altium `UniqueId` or typed sidecar identity.
3. Stable source identity.
4. Natural key within a parent scope.
5. Similarity-based candidate matching.
6. Ambiguous match: hard error requiring resolution.

Structural hashes are fingerprints, not identity. Geometry changes alter the hash, and identical
primitives can collide. Use hashes to test equality and rank candidates, never as the only durable
identity mechanism.

For managed files, plan/apply may add annotations and typed metadata to both the spec and Altium
document. Prefer native IDs where sufficient. Custom metadata must be fully parsed and serialized by
`altium-format`; unknown parameter retention and opaque streams remain forbidden.

Before choosing a metadata carrier, verify with fixtures and reverse-engineered Altium code that GUI
saves preserve it. Candidate carriers include typed document/component parameters or a dedicated,
fully understood stream. If preservation is not proven, use managed external state or native-only
identity.

### 6.1 GUI-save preservation: empirical result (2026-06-21, SchLib / Altium 26.2)

Tested directly. Unknown `|KEY=VALUE|` pairs were injected at four points through the high-level
write path, the file was opened in the Altium GUI, saved (`Ctrl+S`, which re-serializes in place),
and the result was inspected stream-by-stream.

| Injection point | Entity has native UniqueID? | Survived GUI save? |
| --- | --- | --- |
| `FileHeader` document block (raw key) | n/a | No — dropped |
| Component header `RECORD=1` (raw key) | Yes | No — dropped |
| Pie primitive `RECORD=9` (raw key) | No | No — dropped |
| User parameter `RECORD=41` (real `Name=/Text=` object) | n/a | Yes — Altium normalized it and assigned it a fresh UniqueID |

Conclusions that constrain the identity/metadata design:

- Altium round-trips every record through a typed model and **discards any parameter it has no field
  for** on save. This is the same fail-fast/typed-reserialize behavior `altium-format` itself enforces.
- **Native UniqueID presence does not protect an unknown key.** The component header carries a
  UniqueID and still lost its injected key. Survival depends on the key being a *recognized
  first-class object*, not on the host entity having an id.
- The only proven embedded carrier in SchLib is a **real component parameter (`RECORD=41`), and only
  at component scope.** Document-level keys and sub-component primitives cannot carry surviving
  embedded metadata.
- Therefore `MetadataPolicy::ManagedEmbedded` is viable **only for component-scoped metadata via real
  parameters**. Document-level and primitive-level managed metadata must use
  `MetadataPolicy::ManagedExternal` (an external baseline file) or fall back to native-only identity.
- For entities below component scope (pins, graphics) and for document/component headers, **binding
  must rely on native `UniqueId` where it exists, and on structural/natural-key matching where it does
  not.** We cannot persist our own `BindingId` on them.
- Still to confirm for PcbLib, SchDoc, PcbDoc, PrjPcb. The pipeline is the same param-based
  typed-reserialize, so the result is expected to be identical, but each must be verified before any
  format relies on an embedded carrier.

### 6.2 Resolved identity model (2026-06-21)

Native identity coverage is heterogeneous (verified against the high-level API types). The new design
must model identity *source/confidence per entity*, not assume a single id field per entity — that
assumption is the core defect of the current reconciler (one natural-key string used as both identity
*and* apply-address, exact-match only, additive-only, no rename detection, no baseline).

Identity tiers:

| Tier | Entities | Anchor |
| --- | --- | --- |
| 1 — reliable native id | All SchDoc sheet objects; PrjPcb document refs/variants; PcbDoc *named* collections (Net/Component/Polygon/Rule, `id == name`) | `BindingId` ← native `UniqueId` directly; renames are free |
| 2 — present-but-blank | SchLib Component / Pin / Parameter (`unique_id` exists but is often empty) | Component anchored by an embedded `BindingId`; Pin/Parameter use native id when populated, else parent-scoped natural key |
| 3 — keyless | SchLib `PieGraphic`; **all** PcbLib primitives; **all** PcbDoc primitives (their `id` is synthesized at parse, not stored in the file) | No native id and not embeddable — identity exists **only** in the external ledger |

Three fixed decisions:

1. **The keyless structural ledger is designed up front** (covers Tier 3 across PcbDoc/PcbLib/Pie), not
   deferred. The baseline format must support all three tiers from the first release.
2. **Managed SchLib component binding is embedded as a hidden `RECORD=41` parameter** — the only
   GUI-survivable carrier (§6.1). Sub-component and keyless entities are carried by the external ledger
   only. (Reserved parameter name TBD; must be re-confirmed survivable when `is_hidden` and excluded
   from Altium's own param dedup. The visible-parameter case is proven.)
3. **Unmatched + key-changed entities are handled conservatively**: delete + add, surfaced as a
   blocking review item. Exact-id, stable natural-key, and exact-fingerprint/stable-ordinal pairing are
   allowed; similarity/"looks alike" pairing is never performed.

Core types:

```rust
struct BindingId(u128);   // opaque, minted at first bind, never derived from mutable data

enum DocumentLocator {                                   // how a BindingId re-finds its Altium entity
    Native { unique_id: String },                        // Tier 1, and Tier 2 when populated
    NaturalKey { parent: BindingId, key: String },       // Tier 2 fallback
    Structural {                                          // Tier 3 — ledger only
        parent: BindingId,
        collection: CollectionKind,
        ordinal: u32,
        fingerprint: Fingerprint,
    },
}

struct LedgerEntry {
    binding: BindingId,
    parent: Option<BindingId>,
    source: Option<SourceId>,            // authored-side CST node identity
    document: DocumentLocator,           // Altium side
    semantic_fingerprint: Fingerprint,   // EXCLUDES management metadata (the embedded BindingId param)
    revision: Revision,
}
```

Resolution ladder (each run, per entity; stop at first hit):

1. Embedded managed `BindingId` (SchLib component parameter).
2. Native `UniqueId`.
3. Parent-scoped natural key.
4. Ledger structural match: same `(parent, collection, ordinal)`, fingerprint confirms/disambiguates.
5. Ledger exact-fingerprint match within parent (recovers reorder/insert when the ordinal shifted) —
   only when the match is unique.
6. No unique match → fresh `BindingId` for a genuinely new entity; an unmatched baseline counterpart is
   a delete. Any pair that would require similarity guessing is emitted as delete+add and flagged for
   review, never silently bound.

Identity is decoupled from the apply target. `BindingId` is the stable handle; `ResourceAddress`
(parent path + collection + locator) is re-resolved from it each run, so reordering an entity does not
change its identity. Change detection: same `BindingId` resolving on both sides with a differing
semantic fingerprint is an Update; address/ordinal drift with an unchanged fingerprint is a Move. The
embedded `BindingId` parameter is excluded from the semantic fingerprint, so writing it never registers
as a content change (§7).

Consequences accepted:

- Editing a keyless primitive's geometry keeps identity via `(parent, collection, ordinal)`; the
  fingerprint flags the edit as Update. If the collection's count or order also changed so the ordinal
  is ambiguous, the affected primitives fall to delete+add+review rather than being fuzzy-matched.
- First brownfield adoption has no ledger and no embedded ids: bootstrap binds by native id + natural
  key, mints `BindingId`s, and writes the ledger (plus component params where managed/embedded). Keyless
  primitives bind by an ordinal+fingerprint snapshot taken at adoption.

## 7. Synchronization baseline

Every successful managed synchronization records a semantic baseline:

```rust
struct SyncBaseline {
    schema_version: u32,
    semantic_digest: Digest,
    resources: IndexMap<BindingId, ResourceBaseline>,
}

struct ResourceBaseline {
    source_id: Option<SourceId>,
    altium_id: Option<AltiumNativeId>,
    address: ResourceAddress,
    parent: Option<BindingId>,
    semantic_fingerprint: Fingerprint,
    revision: Revision,
}
```

Exclude management metadata itself from semantic fingerprints.

The baseline may be:

- Embedded in both artifacts when the typed carrier is proven and policy permits it.
- Stored in an external versioned state file.
- Absent in native-only mode, in which case operations are two-way adoption/reconciliation and
  conflict detection is necessarily weaker.

## 8. Three-way planning

The planner compares:

```text
Base -> current resolved source snapshot
Base -> current Altium document snapshot
```

Per resource it classifies:

```rust
enum ChangeDisposition {
    SourceOnly,
    DocumentOnly,
    SameChange,
    Conflict,
    Unchanged,
}
```

Command direction selects the default resolution policy:

- `compile`: project source-side changes toward Altium.
- `dump`: project Altium-side changes toward source.
- `SameChange`: converge metadata/baselines without repeating the semantic edit.
- `Conflict`: block apply until explicitly resolved.
- Metadata/binding changes may update both artifacts in either direction.

First adoption has no baseline. The user chooses the bootstrap direction:

- First `dump`: Altium establishes the source representation and bindings.
- First `compile`: source establishes the document representation and bindings.

The initial plan must show all semantic and metadata changes before writing either artifact.

## 9. Managed scope and absence semantics

Do not encode all of these meanings as `Option::None`:

- Inherit from a template/default.
- Leave the current target value unchanged.
- Reset to the format default.
- Clear/delete the value.
- Field is unsupported or unmanaged.

Model them explicitly at the authored boundary. After resolution, a managed artifact snapshot should
be concrete.

```rust
struct ManagedScope {
    entity_kinds: ManagedKinds,
    fields: ManagedFields,
    allow_delete: bool,
}
```

Delete only within a completely representable, authoritative managed scope. If a document section
cannot be represented, planning must return a blocking unsupported diagnostic. It must not silently
preserve the section while claiming convergence, and it must never delete content it cannot model.

## 10. Executable plan model

The current ECO is presentation-oriented and should be replaced.

```rust
struct PlanBundle {
    format_version: u32,
    domain: Domain,
    operation: Operation,
    authority: Authority,
    metadata_policy: MetadataPolicy,
    managed_scope: ManagedScope,

    input_preconditions: Vec<ArtifactPrecondition>,
    semantic_changes: Vec<SemanticChange>,
    patches: Vec<ArtifactPatch>,
    diagnostics: Vec<PlanDiagnostic>,
}
```

The plan must be versioned and deserializable. It contains hashes/preconditions for every source and
target artifact it may write.

### 10.1 Semantic changes

Semantic changes power the ECO renderer:

```rust
enum SemanticAction {
    Create,
    Update,
    Rename,
    Move,
    Delete,
    Replace,
    MetadataOnly,
    Unchanged,
    Conflict,
}
```

They use typed or structured values internally. Formatting into mils, layer names, colors, and text
belongs in the renderer.

### 10.2 Typed artifact patches

```rust
enum ArtifactPatch {
    SchLibDocument(SchLibPatch),
    PcbLibDocument(PcbLibPatch),
    SchDocDocument(SchDocPatch),
    PcbDocDocument(PcbDocPatch),
    PrjPcbDocument(PrjPcbPatch),
    SpecSource(SpecSourcePatch),
    Baseline(BaselinePatch),
}
```

Prefer replacing high-level aggregates rather than implementing property setters for every field:

```rust
enum PatchOp<T> {
    Create {
        address: ResourceAddress,
        value: T,
    },
    Replace {
        address: ResourceAddress,
        expected: Fingerprint,
        value: T,
    },
    Delete {
        address: ResourceAddress,
        expected: Fingerprint,
    },
}
```

For example, changing a SchLib pin replaces the containing high-level component through
`update_component`. This aligns patch granularity with the high-level API and avoids a second field-
by-field executor.

## 11. Source patches

`dump apply` executes structured CST edits:

```rust
enum SpecEdit {
    InsertBlock {
        parent: SourceId,
        block: IntentBlock,
    },
    DeleteBlock {
        id: SourceId,
    },
    SetProperty {
        id: SourceId,
        key: PropertyKey,
        value: Expr,
    },
    RemoveProperty {
        id: SourceId,
        key: PropertyKey,
    },
    SetAnnotation {
        id: SourceId,
        binding: BindingMetadata,
    },
}
```

First-time dump builds a new CST and formats it canonically. Subsequent dumps edit the existing CST.
Malformed existing source is a hard error. Never warn and overwrite it.

When greenfield source uses an imported definition, dumping GUI changes should emit instance
overrides rather than re-inline the entire materialized definition. Brownfield source may represent
the complete inline Altium state. That behavior is selected by authority and the existing authored
construct, not by a heuristic mode switch.

## 12. Plan application and filesystem transaction

Applying a plan:

1. Read all affected source and target artifacts.
2. Verify every content hash and resource precondition.
3. Clone/project documents into mutable high-level API objects.
4. Execute typed document and source patches in memory.
5. Run all structural and semantic validators.
6. Serialize every output to temporary files.
7. Reopen serialized outputs and validate them again.
8. Prepare the updated synchronization baseline.
9. Commit prepared files using renames plus a small transaction journal.
10. Remove the journal only after every artifact is committed.

True atomic replacement across multiple files and filesystems cannot be guaranteed. Use a recovery
journal instead of claiming perfect atomicity. A failed operation must be recoverable to either the
old or new complete set.

## 13. CLI surface

Infer domain from extensions; do not require five redundant outer domain command groups unless a
future domain genuinely needs a distinct surface.

```text
# Spec -> Altium
altium compile plan  design.schdoc-spec --target design.SchDoc --out plan.json
altium compile apply --plan plan.json

# Altium -> spec
altium dump plan  design.SchDoc --spec design.schdoc-spec --out plan.json
altium dump apply --plan plan.json
```

Optional convenience mode:

```text
altium compile apply design.schdoc-spec --target design.SchDoc --auto-approve
altium dump apply design.SchDoc --spec design.schdoc-spec --auto-approve
```

The strongest workflow uses saved plans. `apply --plan` executes exactly the reviewed artifact after
checking preconditions.

Recommended exit codes:

- `0`: converged/no changes.
- `2`: plan contains applicable changes.
- `1`: error, unsupported content, or conflict.

Plan output must group changes by artifact because one operation may update the spec, Altium file,
and baseline together.

## 14. Crate boundaries

Recommended initial split:

```text
altium-format
  low-level parsing/writing
  high-level document APIs
  invariant validation
  no spec-language or planning logic

altium-spec-lang
  lossless CST
  parser and formatter
  authored intent model
  imports and semantic compilation

altium-reconcile
  concrete artifact snapshots
  identity and bindings
  baseline state
  three-way planner
  typed plan and patches
  semantic ECO rendering
  conflict classification/resolution

altium-cli
  command parsing
  filesystem transactions
  plan storage and reporting
```

Dependency direction:

```text
altium-format-types
        |
        +--> altium-format
        +--> altium-spec-lang
                    \
                     +--> altium-reconcile --> altium-cli
                    /
          altium-format
```

`altium-reconcile` may depend on both `altium-format` and `altium-spec-lang`; neither depends on the
reconciler.

Avoid splitting into additional crates until a real dependency boundary demands it.

## 15. Architecture options and tradeoffs

### Option A: high-level Altium API as the only semantic model

Pros:

- Fastest implementation.
- Excellent brownfield fidelity.
- Minimal projection code.
- Spec becomes a direct serialization of Altium-shaped types.

Cons:

- Vendor-shaped authored language.
- Poor representation of imports, generators, definitions, occurrences, and constraints.
- Weak cross-domain semantics.
- Encourages conflating omitted intent with concrete defaults.

Good fit if the only goal is editing Altium artifacts textually. Not recommended as the complete
long-term architecture.

### Option B: one new vendor-neutral design graph

Pros:

- Clean long-term multi-EDA architecture.
- Explicit definitions, occurrences, connectivity, and cross-domain links.
- Good native model for future UI, analysis, placement, and routing.

Cons:

- Largest initial scope.
- Hard to map all brownfield Altium detail without format leakage.
- Risks blocking reliable plan/apply on broader product-model decisions.
- Existing graph drafts permit opaque preservation and therefore conflict with current rules.

Do not make this a prerequisite for the first replacement workflow.

### Option C: authored intent plus concrete Altium snapshot

Pros:

- Preserves ergonomic greenfield abstractions.
- Preserves full concrete brownfield state.
- Both sides compare in a complete resolved snapshot.
- Allows the future design graph to be inserted later.
- Keeps Altium-specific details at the artifact boundary.

Cons:

- Two models and a required elaboration/projection layer.
- Must prove that compiling intent and importing documents converge to equivalent snapshots.
- More initial design work than Option A.

Recommended option.

### Option D: authored intent plus design graph plus Altium snapshot

Pros:

- Best eventual separation of authored intent, canonical design semantics, and vendor artifact.
- Strongest multi-tool and cross-domain future.

Cons:

- Three models and two major projections.
- Highest consistency and testing burden.
- Too large for the first replacement slice.

Treat as an evolutionary destination, not the starting point.

## 16. Implementation strategy

Build complete vertical domain slices. Do not build one phase across all five formats and leave every
workflow partially functional.

### Foundation

1. Freeze semantics for authority, metadata policy, managed scope, identity, baseline, conflicts,
   and saved plans.
2. Define plan schema, typed preconditions, semantic changes, and patch traits.
3. Implement the lossless source CST and structured edit API.
4. Implement filesystem staging, validation, and recovery journal support.

### Domain slices

1. SchLib as the reference implementation.
2. PcbLib.
3. PrjPcb.
4. SchDoc, including full inline component children and greenfield overrides.
5. PcbDoc last because primitive identity, ownership, collection mutation, and high-level API
   completeness are the hardest.

For each domain, implement:

1. Source intent compilation.
2. Intent-to-snapshot elaboration.
3. Document-to-snapshot projection.
4. Identity extraction and binding.
5. Three-way semantic diff.
6. Typed document patch lowering.
7. Structured source patch lowering.
8. Plan rendering.
9. Apply, serialization, reopen, and invariant validation.
10. Independent completeness review before proceeding to the next slice.

Remove the old machinery when the replacement slice covers its domain. Do not maintain two active
planners/executors for the same domain longer than necessary.

## 17. Required laws and acceptance criteria

Every domain must satisfy:

```text
project(document) == snapshot
materialize(snapshot) then project == snapshot
apply(plan(base, source, document)) == planned after-state
replanning after apply produces no changes
applying a stale plan fails
independent same-resource edits produce Conflict
unsupported or unrepresentable content blocks planning
```

Additional requirements:

- Plan and apply cannot disagree because apply consumes only plan patches.
- Dump and compile both support `plan` and `apply`.
- Both operations may show and update multiple artifacts.
- Brownfield inline content round-trips losslessly.
- Greenfield imported definitions materialize as bases plus explicit overrides.
- Managed brownfield may add typed annotations and metadata.
- Native-only brownfield never adds tool metadata.
- Deletion occurs only inside explicit, fully supported managed scopes.
- Ambiguous identity is a hard error.
- Malformed source is never overwritten.
- No parser, planner, projection, or apply path silently skips input.
- No opaque/raw/unparsed format data is introduced.
- Every fallible format boundary returns contextual errors.

## 18. Testing strategy

Use layered tests:

- Unit tests for CST editing, identity, fingerprints, matching, conflict classification, and patch
  preconditions.
- Model-law tests for elaboration/projection convergence.
- Per-domain plan/apply tests with small programmatic documents that do not require fixtures.
- Saved-plan staleness and schema-version tests.
- Transaction recovery tests with injected failures between commits.
- Fixture-dependent roundtrip tests behind `test-fixtures` only.
- Property tests behind `proptest` only.

Do not weaken comparisons to normalize unexplained differences. Supported upgrades must be explicit,
typed, documented, and visible in reports.

## 19. Separate low-level CFB verification

Keep semantic planning separate from serialized CFB verification.

- The user-facing ECO describes semantic resource changes.
- The typed plan describes executable source/document patches.
- `cfb diff --semantic` remains an agent/developer tool for validating serialization behavior.

An optional compile-plan verification mode may dry-run the plan and compare before/after CFB output,
but that is a parallel report, not part of the semantic change algebra and not proof of correctness by
itself.

## 20. Immediate next decisions

Before implementation begins, explicitly decide:

1. Whether the existing surface syntax is retained, revised, or replaced.
2. Which lossless CST technology/pattern to use.
3. Exact `BindingId`, `SourceId`, and resource-path formats.
4. Embedded metadata carriers per Altium document type, after GUI preservation validation. SchLib is
   resolved (see §6.1): the only surviving embedded carrier is a real component parameter
   (`RECORD=41`) at component scope; everything else must use external state. PcbLib/SchDoc/PcbDoc/
   PrjPcb still need the same validation.
5. External baseline file name and versioning. §6.1 makes this mandatory rather than optional: any
   metadata below component scope (pins, primitives) or at document scope cannot be embedded, so the
   external baseline is the primary carrier, not a fallback.
6. Whether managed scope is declared in source, project configuration, CLI policy, or a combination.
7. Saved-plan compatibility/versioning policy.
8. Conflict-resolution UX and whether the first version only reports conflicts.
9. Aggregate patch boundaries for each high-level API.
10. Which current spec-language features belong in the first SchLib vertical slice.

Until these are settled, do not begin another incremental reconciler/executor patch series.
