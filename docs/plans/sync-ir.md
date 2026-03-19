# SyncSnapshot IR: Annotation-Based Spec Sync

## Overview

Implement a bidirectional synchronization system between SchDoc-spec and PcbDoc-spec files using a common `SyncSnapshot` intermediate representation. The system extends the spec language with Rust-style block annotations (`#[annotation(id = "...", ...)]`) for persistent identity using Altium-style 8-character short IDs. A multi-phase executor/compiler pipeline (Parse → Compile → Validate → Resolve → Project) resolves sparse specs against sane defaults and library lookups to produce the final IR. The sync system integrates with the existing autoplacer constraint types, which go beyond standard Altium ECO types to include edge placement, directional constraints, region containment, and fixed positions.

Approach B (Modular) selected: new modules (`annotation.rs`, `sync.rs`, `resolver.rs`, `validator.rs`) within the `altium-format-spec` crate, keeping sync logic co-located with the spec model it projects from.

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|----------------|
| Modular approach (B) over minimal (A) or new crate (C) | compiler.rs is already ~23K LOC → adding sync+annotation logic there makes it harder to navigate → new modules enforce phase boundaries → but sync needs to mutate SpecModel for back-annotation → same crate avoids circular dependency that (C) would create |
| Predefined annotation keys only (id, stable, group) | Arbitrary keys cannot be validated at compile time → typos silently ignored → predefined keys catch errors early → if future extension needed, add new predefined keys rather than escape hatch |
| Auto-generate IDs on dump/save | Sync requires every block to have an ID → manual annotation is friction → auto-generation makes sync work from day one → IDs are stable across rewrites (if block identity preserved) → matches Altium's auto-assigned UniqueID behavior |
| Altium-style 8-char short IDs | Altium uses 8-char alphanumeric UniqueIDs → human-readable in diffs → compact in text → compatible with existing Altium document UIDs if we ever bridge spec↔document identity |
| Designator-based matching as primary | Specs are text files without persistent UUIDs initially → designators are the natural identity in specs → Altium itself falls back to designator matching (eMapByDesignator) when UIDs missing → annotation IDs provide secondary identity for rename detection |
| SyncSnapshot is ephemeral (not persisted) | Altium's ECO objects are transient in-memory → recomputing is cheap → eliminates staleness → for three-way merge, base snapshot needs persistence but that's Phase 3 |
| Multi-phase pipeline (5 phases) | Monolithic compile+execute conflates concerns → validation errors surface late → library resolution is a distinct concern from parsing → explicit phases enable targeted testing and better error messages |
| Sane defaults in compiler phase | Sparse specs should "just work" → omitted fields get sensible values from schema defaults → document-backed defaults (preserve existing) handled by executor's `preserve_missing` mode → separation: compiler provides schema defaults, executor provides document defaults |
| Proptest for diff algorithm | Diff has algebraic invariants (idempotency, symmetry, empty-diff-on-equal) → property tests cover wide input space → matches existing project proptest usage |
| Spec-only integration tests | Sync operates on spec models, not binary documents → synthetic spec text is sufficient → avoids test-fixtures feature gate → fast, deterministic CI |
| Static execution ordering over conflict detection | Altium uses a 161-element ModificationOrder array → removes before adds, containers before members → ordering prevents conflicts without complex dependency analysis → simpler and proven at scale |
| 3-phase ordering suffices for Phase 1 | PcbDoc apply only mutates component and net lists → no cross-reference indices exist in the spec model → Altium's 161-element array handles fine-grained dependency ordering within binary document objects → spec-level entities have no such dependencies in Phase 1 → 3-phase (Remove/Update/Add) is sufficient |
| SyncSnapshot uses IndexMap over HashMap | Insertion-order preservation ensures deterministic diff output → changes listed in spec declaration order → required for reproducible ECO reports and idempotent test assertions → HashMap would make diff output non-deterministic breaking test assertions |
| Net color not synchronized in Phase 1 | SchDoc specs typically do not specify net colors → synced nets receive no explicit color (None) → letting Altium apply system defaults avoids wrong-color display → Phase 3 may add color sync if needed |
| `rand` crate for ID generation | Must verify `rand` is already a workspace dependency (check Cargo.lock) → match existing version to avoid workspace pin conflicts → if `rand` unavailable, fallback: use `std::time::SystemTime` + `std::hash` as entropy source |
| Duplicate ID detection is dual-layer | Compiler detects within-file duplicates during incremental compilation (fast-fail) → validator detects cross-file duplicates in Phase 3 (authoritative) → both checks are intentional → document rationale in `annotation.rs` comment |
| PcbDocSpec is single-board for Phase 1 sync | Multi-board specs would cause silent correctness failure (only boards[0] synced) → return error if `boards.len() != 1` → multi-board support deferred to future phase |
| `seen_ids` scope is per-file | `seen_ids: HashSet<String>` is constructed fresh per top-level compile call (one set per spec file) → not shared across file compilations → cross-file duplicate detection is the validator's responsibility (Phase 3) |
| Projection fails on dangling refs | Net-pin refs to unknown designators are hard errors in projection → fail fast per CLAUDE.md → no silent data loss from dropped connectivity → validator catches structural issues before projection, but projection also validates its inputs |
| Resolver fails on missing referenced libraries | If a component explicitly references a library that can't be found, that's a hard error → "cannot resolve library X" → no silent footprint loss → only components with NO library reference get None footprint (valid case for bare designators) |
| Pin changes are not generated in Phase 1 diff | PcbDoc spec lacks pin-level connectivity → diff must not produce AddPin/RemovePin/UpdatePin → if somehow generated, apply returns hard error → never silently drop connectivity changes |
| `filter_changes` direction parameter semantics | The `direction` parameter is the caller's active sync direction (Forward when calling from `spec sync --forward`, Back from back-annotation) → `filter_changes` keeps a `FieldChange` only if the policy's direction for that property is `Bidirectional` OR equals `direction` → `SyncDirection::None` always excludes the field regardless of `direction` |
| `filter_changes` must hard-error on pin variants | `filter_changes()` MUST return `Err` when it encounters any `AddPin`/`RemovePin`/`UpdatePin` variant in Phase 1 → these should never be generated by the diff but if they appear, they must not be silently stripped → silent stripping bypasses apply()'s guard and violates "never silently drop connectivity changes" invariant |
| Footprint direction is None in Phase 1 forward SyncPolicy | SchDoc projection always yields `footprint: None` (SchDoc specs do not assign footprints) → syncing this None forward would silently clear all PcbDoc footprint assignments → footprint sync deferred to Phase 2 resolver → same reasoning as net color exclusion |
| Phase 1 forward SyncPolicy property mapping | Explicit per-property directions for `--forward`: `comment: Forward`, `footprint: None` (always None from SchDoc), `source_library: None` (always None from SchDoc), `parameters: None`, `net_name: Forward`, `net_color: None` (excluded Phase 1), `pin_net_assignment: None` (excluded Phase 1), `component_location: None` (never synced) → user-confirmed |
| Parameters policy is None in Phase 1 forward sync | PcbDocComponentSpec does not carry a parameters field → apply returns NotSupported for parameter fields → CLI sets parameters: None to prevent hitting an unreachable error path → when PcbDoc spec gains a parameters field, change this policy to Forward |
| ConstraintKind initial variants | Initial variant list to be confirmed with user before M8 implementation — wrong names cause silent parse failures in spec files → variants must be user-specified (Tier 1) — M8 acceptance criteria must NOT hardcode variant names until confirmed |
| CLI spec write-back uses atomic temp+rename | Direct overwrite truncates file on interrupted write → spec corruption → wrong future sync results → temp-file-then-rename either completes fully or leaves original intact → matches fail-fast principle |
| Validator return type carries warnings on success path | `validate_*_spec()` returns `Result<Vec<SpecError>, Vec<SpecError>>` where `Ok(warnings)` carries non-fatal warnings and `Err(errors)` carries hard errors → CLI prints warnings from `Ok(warnings)` to stderr before proceeding → CLI converts `Err(errors)` to single `anyhow::Error` with all errors joined → `.with_context()` applied at Vec→anyhow boundary → warnings never silently dropped |
| `SyncPolicy` is always explicitly constructed | CLI always constructs an explicit `SyncPolicy` with named direction per property → `SyncPolicy::default()` is not used in the sync pipeline → avoids accidental wrong-direction sync from implicit defaults |
| SpecError gains Severity field via builder | Validator needs to report warnings (e.g., unresolved pin refs from missing libraries) alongside hard errors → SpecError gains `severity: Severity` field defaulting to `Severity::Error` for all existing constructors → new `with_severity(Severity) -> Self` builder method for explicit override → validator creates warnings via `SpecError::no_span(...).with_severity(Severity::Warning)` → `render()` prefixes output with `warning[...]` vs `error[...]` → existing call sites unchanged (default Error preserves existing behavior) |
| ConstraintSpec.kind is a typed enum | Constraint kinds must catch typos at compile time (same rationale as predefined annotation keys) → ConstraintKind enum with known variants → requires updating enum for new kinds but prevents silent acceptance of misspelled kinds |
| Net matching by name only | Altium matches nets by DM_FullNetName() with no net UUID → specs identify nets by name → name-based matching is structurally identical to Altium |
| Annotation on constraint blocks | Placement constraints (separate, group, directional) need post-solve reporting → annotation IDs enable constraint-to-result tracing → fine-grained debugging |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| Approach A (Minimal) | compiler.rs at 23K LOC → adding sync+annotation+validation logic pushes it further → phase boundaries become implicit, not testable → no clean module for sync-specific proptest |
| Approach C (New crate) | Sync must mutate SpecModel for back-annotation → cross-crate mutation requires pub API exposure → leaks spec internals → workspace publish/version coordination overhead for tightly coupled code |
| Arbitrary annotation key-value pairs | Cannot validate at compile time → typos in keys silently ignored → "stabl" instead of "stable" produces no error → predefined keys catch this |
| UUID-based identity (128-bit) | Verbose in text files → not human-readable in diffs → overkill for spec-level identity → Altium's own 8-char format is sufficient and compatible |
| Manual annotation only | Requires user to annotate every block before sync works → high friction → defeats "sync from day one" goal → auto-generation removes barrier |
| Persistent SyncSnapshot file | Adds new file format to maintain → staleness risk if user edits spec without re-syncing → recomputing is O(ms) so caching provides no meaningful benefit |

### Constraints & Assumptions

- Rust 2021 edition, workspace build with `cargo test --workspace`
- Existing spec pipeline: lexer → parser → compiler → executor/reconciler/dump
- `#` is not currently a token in the lexer (available for annotation syntax)
- `[` and `]` are already tokens (used in array syntax)
- Altium UniqueID format: 8 characters from `[A-Z][0-9]` (uppercase alpha + digits, 36^8 ≈ 2.8 trillion combinations)
- Placement constraints (`UserConstraint` enum in autopcb-placement) are already used by the spec executor via placement_bridge.rs
- Default conventions applied: `testing` (property-based preferred), `file-creation` (new modules at module boundary)

### Known Risks

| Risk | Mitigation | Anchor |
|------|-----------|--------|
| Annotation syntax conflicts with future spec syntax | `#[` is a distinctive two-token prefix unlikely to collide → Rust has proven this prefix stable for a decade | — |
| Short ID collisions | 36^8 ≈ 2.8T combinations → collision probability negligible for spec-scale files (< 10K blocks) → Phase 3 validator detects duplicates | — |
| Parser complexity from annotation prefix | Annotation parsing is a single `parse_annotation()` method called before each block declaration → localized complexity → well-tested pattern from Rust's own parser | — |
| Diff algorithm correctness for edge cases | Proptest with diff invariants (idempotency, symmetry) → catches edge cases mechanically | — |
| Footprint resolution requires SchLib access | Phase 1 leaves footprint None in SchDoc projection → forward sync still works for component/net sync → footprint resolution deferred to Phase 2 resolver | — |

## Invisible Knowledge

### Architecture

```
Spec Text (.schdoc-spec / .pcbdoc-spec)
    │
    ▼ Phase 1: PARSE
  Tokens → AST (with BlockAnnotation nodes)
    │
    ▼ Phase 2: COMPILE
  SpecModel (with resolved annotations, sane defaults)
    │
    ▼ Phase 3: VALIDATE
  Consistency checks (duplicate IDs, dangling refs)
    │
    ▼ Phase 4: RESOLVE
  SpecModel + Libraries → ResolvedSpec (footprint→designator maps)
    │
    ├──▶ Phase 5a: PROJECT (Sync)
    │    SyncSnapshot → diff → SyncChanges → apply to target spec
    │
    ├──▶ Phase 5b: PROJECT (Placement)
    │    PlacementSpec → UserConstraints → autoplacer
    │
    └──▶ Phase 5c: PROJECT (ECO)
         SpecModel vs Document → EngineeringChangeOrder
```

### Data Flow

```
SchDoc-spec ──project──▶ SyncSnapshot ◀──project── PcbDoc-spec
                              │
                         diff_snapshots()
                              │
                              ▼
                        Vec<SyncChange>
                              │
                    ┌─────────┴─────────┐
                    ▼                   ▼
            apply to PcbDoc-spec   apply to SchDoc-spec
            (forward sync)         (back-annotation)
```

### Why This Structure

The sync IR (`SyncSnapshot`) is separate from the reconciler's ECO because they solve different problems:
- **Reconciler** (existing): diffs a spec against a *binary Altium document* → produces changes to apply to the document
- **Sync** (new): diffs two *specs* against each other via a common projection → produces changes to apply to a spec

Both produce `EntityChange`-style outputs but operate on different input types. The reconciler continues to handle spec-to-document application; the sync system handles spec-to-spec synchronization.

### Invariants

1. Every block in a spec file has a unique annotation ID after dump/save (auto-generated if not present)
2. Annotation IDs are stable across spec rewrites — same block identity → same ID
3. `diff_snapshots(a, a)` always produces an empty changeset (reflexive)
4. `apply(diff(source, target), target)` produces a state where `diff(source, target')` is empty (convergent)
5. SyncSnapshot projection is side-effect-free (no document mutation) but IS fallible — returns Result on invalid input
6. Annotation `stable: true` blocks are skipped by the executor during apply

### Tradeoffs

- **Designator-first matching over ID-first**: Simpler for Phase 1, but renames look like delete+add until Phase 3 three-way merge uses annotation IDs to detect identity preservation across renames
- **Auto-ID generation**: Convenience over control — users who want specific IDs can set them manually; auto-generated IDs may change if block identity is ambiguous during dump
- **Predefined keys only**: Safety over flexibility — no escape hatch for custom metadata, but prevents the annotation system from becoming an untyped property bag

## Milestones

### Milestone 1: Annotation Syntax — Lexer, Parser, AST

**Files**:
- `crates/altium-format-spec/src/lexer.rs`
- `crates/altium-format-spec/src/parser.rs`
- `crates/altium-format-spec/src/ast.rs`

**Flags**: `needs-rationale`

**Requirements**:
- Lexer recognizes `#` as `TokenKind::Hash` (if not already present)
- Parser implements `parse_annotation()` that consumes `#[annotation(key = value, ...)]`
- AST gains `BlockAnnotation` struct with `id: Option<Spanned<String>>`, `stable: Option<Spanned<bool>>`, `group: Option<Spanned<String>>`
- All block declaration AST nodes (`ComponentDecl`, `FootprintDecl`, `NetDecl`, `PlacementDecl`, `BoardDecl`, `SheetDecl`, `RuleDecl`, `ClassDecl`, `PolygonDecl`, `PlacementPlaceDecl`) gain `annotation: Option<Spanned<BlockAnnotation>>` field
- Parser calls `parse_annotation()` before each block declaration when `#` token is found at position
- Unknown annotation keys produce a compile error
- Missing `id` key in annotation is valid (auto-generated later)

**Acceptance Criteria**:
- `#[annotation(id = "AB12CD34")] component R1 { ... }` parses to AST with annotation attached
- `#[annotation(id = "AB12CD34", stable = true, group = "power")] net VCC { ... }` parses correctly
- `#[annotation()] component R1 { ... }` parses (empty annotation, ID generated later)
- `#[annotation(unknown_key = "x")]` produces parse error "unknown annotation key 'unknown_key'"
- Annotation without block declaration produces parse error
- Existing specs without annotations continue to parse unchanged
- Update STATUS.md to reflect annotation syntax support

**Tests**:
- **Test files**: `crates/altium-format-spec/src/parser.rs` (inline `#[cfg(test)]`)
- **Test type**: example-based unit tests
- **Backing**: user-specified (annotation syntax is new, no existing patterns)
- **Scenarios**:
  - Normal: annotation with all keys, annotation with id only, empty annotation
  - Edge: annotation on every block type, multiple annotations in sequence
  - Error: unknown key, missing brackets, annotation without block

**Code Intent**:
- `ast.rs`: Add `BlockAnnotation` struct with three `Option<Spanned<T>>` fields (id: String, stable: bool, group: String). Add `AnnotationKey` enum (Id, Stable, Group). Add `annotation: Option<Spanned<BlockAnnotation>>` field to all `*Decl` structs that represent named blocks.
- `lexer.rs`: Add `Hash` variant to `TokenKind` if not present. Tokenize `#` as `Hash`.
- `parser.rs`: Add `parse_annotation(&mut self) -> Result<Option<Spanned<BlockAnnotation>>>` method. Match `#` token → expect `[` → expect ident `annotation` → expect `(` → parse comma-separated key=value pairs → validate keys against `AnnotationKey` enum → expect `)` → expect `]`. Update all `parse_*_decl()` methods to call `parse_annotation()` first and attach result.

**Code Changes**: _(Developer fills diffs)_

---

### Milestone 2: Annotation Compilation & ID Generation

**Files**:
- `crates/altium-format-spec/src/annotation.rs` _(new)_
- `crates/altium-format-spec/src/compiler.rs`
- `crates/altium-format-spec/src/model.rs`
- `crates/altium-format-spec/src/lib.rs`

**Flags**: `needs-rationale`

**Requirements**:
- New `annotation.rs` module with `generate_short_id()`, `validate_short_id()`, and `CompiledAnnotation` struct
- Short IDs are 8 characters from `[A-Z0-9]` (uppercase + digits)
- `CompiledAnnotation` has fields: `id: String`, `stable: bool` (default false), `group: Option<String>`
- All model spec types gain `annotation: Option<CompiledAnnotation>` field
- Compiler extracts `BlockAnnotation` from AST, validates ID format, compiles to `CompiledAnnotation`
- If annotation has no `id`, compiler auto-generates one via `generate_short_id()`
- Duplicate IDs within a spec file produce a compile error

**Acceptance Criteria**:
- `#[annotation(id = "AB12CD34")]` compiles to `CompiledAnnotation { id: "AB12CD34", stable: false, group: None }`
- `#[annotation(stable = true)]` compiles with auto-generated 8-char ID
- `#[annotation(id = "short")]` produces error "invalid short ID: must be 8 alphanumeric characters"
- `#[annotation(id = "ab12cd34")]` produces error (lowercase not allowed — Altium uses uppercase)
- Two blocks with same ID produce error "duplicate annotation ID 'AB12CD34'"
- `CompiledAnnotation` is available on all compiled model types
- Update STATUS.md to reflect annotation compilation and ID generation

**Tests**:
- **Test files**: `crates/altium-format-spec/src/annotation.rs` (inline `#[cfg(test)]`)
- **Test type**: property-based (proptest) for ID generation/validation, example-based for compilation
- **Backing**: user-specified
- **Scenarios**:
  - Normal: valid ID passes validation, auto-generated IDs are valid format
  - Property: all generated IDs match `[A-Z0-9]{8}` regex, no two sequential calls produce same ID
  - Edge: empty string, 7-char, 9-char, lowercase, special characters
  - Error: invalid format, duplicate IDs
- **Note**: All proptest blocks must be gated with `#[cfg(feature = "proptest")]` per CLAUDE.md

**Code Intent**:
- Pre-implementation: check `crates/altium-format-spec/Cargo.toml` and `Cargo.lock` for existing `rand` dependency; match existing workspace version to avoid pin conflicts (see Decision Log "rand crate for ID generation").
- New `annotation.rs`: `generate_short_id() -> String` using `rand` crate (verified version). `validate_short_id(&str) -> Result<(), String>`. `CompiledAnnotation` struct. Re-export in `lib.rs`.
- `model.rs`: Add `annotation: Option<CompiledAnnotation>` to `ComponentSpec`, `FootprintSpec`, `SchDocComponentSpec`, `NetSpec`, `PowerSpec`, `PcbDocComponentSpec`, `PcbDocNetSpec`, `PcbDocPolygonSpec`, `PcbDocRuleSpec`, `PcbDocClassSpec`, `PcbDocDifferentialPairSpec`, `BoardSpec`, `SheetSpec`, `PlacementSpec`, `PlacementPlaceSpec`.
- `compiler.rs`: Add `compile_annotation(ast_ann: &BlockAnnotation, seen_ids: &mut HashSet<String>) -> Result<CompiledAnnotation>`. Call from each `compile_*_decl()` function. `seen_ids: HashSet<String>` is constructed fresh per top-level compile call (one set per spec file — see Decision Log "seen_ids scope is per-file"). Do not share across file compilations.

**Code Changes**: _(Developer fills diffs)_

---

### Milestone 3: Annotation in Dump & Formatter

**Files**:
- `crates/altium-format-spec/src/dump.rs`
- `crates/altium-format-spec/src/formatter.rs`

**Requirements**:
- Dump functions emit `#[annotation(id = "...")]` before each block declaration
- When dumping a document that has no existing spec, generate fresh IDs for all blocks
- When re-dumping a spec that already has annotations, preserve existing IDs
- Formatter handles annotation lines: `#[annotation(...)]` on its own line, followed by the block declaration
- Annotation line indentation matches the block it decorates

**Acceptance Criteria**:
- `dump_schlib()` produces spec text with `#[annotation(id = "...")]` on every component
- `dump_pcbdoc()` produces spec text with annotations on every component, net, polygon, rule
- Round-trip: parse spec with annotations → compile → dump → parse again → same annotation IDs
- Formatter preserves annotation line before block, with consistent indentation
- Auto-generated IDs differ between blocks (no collisions in a single dump)
- Update STATUS.md to reflect annotation dump format support

**Tests**:
- **Test files**: `crates/altium-format-spec/src/dump.rs` (inline `#[cfg(test)]`)
- **Test type**: example-based, round-trip
- **Backing**: user-specified
- **Scenarios**:
  - Normal: dump SchLib with 3 components → 3 unique annotation IDs
  - Round-trip: spec text → parse → compile → dump → parse → compile → same IDs
  - Edge: empty document produces empty spec (no annotations needed)

**Code Intent**:
- `dump.rs`: Before each `write!` that emits a block keyword (component, footprint, net, etc.), emit `#[annotation(id = "...")]` line. For first-time dumps, call `generate_short_id()`. For re-dumps, read existing annotation ID from the model.
- `formatter.rs`: Add annotation formatting rule: when `#[` is encountered at line start, format as standalone line. Next line is the block declaration at same indent level.

**Code Changes**: _(Developer fills diffs)_

---

### Milestone 4: SyncSnapshot IR & Projection

**Files**:
- `crates/altium-format-spec/src/sync.rs` _(new)_
- `crates/altium-format-spec/src/lib.rs`

**Flags**: `complex-algorithm`

**Requirements**:
- `SyncSnapshot` struct with `components: IndexMap<String, SyncComponent>`, `nets: IndexMap<String, SyncNet>`
- `SyncComponent`: designator, comment, footprint, source_library, parameters (IndexMap), pins (IndexMap of SyncPin), annotation_id
- `SyncPin`: designator, net (Option)
- `SyncNet`: name, color, pins (Vec of component+pin tuples), annotation_id
- `project_schdoc_spec(spec: &SchDocSpec) -> Result<SyncSnapshot, SpecError>`: projects SchDoc spec to snapshot
  - Components from sheets[].components keyed by designator
  - Nets from sheets[].nets → populate pin.net fields on components
  - Powers from sheets[].powers → also populate pin.net fields
  - **Fails hard** on dangling net-pin references (net references non-existent component → error with context)
  - **Fails hard** on duplicate designators across sheets
- `project_pcbdoc_spec(spec: &PcbDocSpec) -> Result<SyncSnapshot, SpecError>`: projects PcbDoc spec to snapshot
  - Components from boards[].components keyed by designator
  - Nets from boards[].nets keyed by name
  - Pins left empty (PcbDoc spec doesn't have pin-level connectivity yet)
  - **Fails hard** on duplicate designators, duplicate net names
- Sane defaults: missing comment → None (not empty string), missing footprint → None, missing parameters → empty map, missing net color → None (Altium uses system default)

**Acceptance Criteria**:
- SchDoc spec with 2 components and 1 net → SyncSnapshot with 2 components, 1 net, correct pin-net assignments
- PcbDoc spec with 3 components and 2 nets → SyncSnapshot with 3 components, 2 nets, empty pins
- Empty spec → empty snapshot (no components, no nets)
- Projection is side-effect-free but returns `Result` — dangling refs are hard errors
- Annotation IDs from compiled model appear in snapshot's annotation_id fields
- Update STATUS.md to reflect SyncSnapshot IR and projection

**Tests**:
- **Test files**: `crates/altium-format-spec/src/sync.rs` (inline `#[cfg(test)]`)
- **Test type**: property-based (proptest) for projection invariants, example-based for structure
- **Backing**: user-specified
- **Scenarios**:
  - Normal: SchDoc with components and nets, PcbDoc with components and nets
  - Property: projection preserves component count, net count, all designators present in output
  - Edge: empty spec, spec with component but no nets, spec with nets referencing missing components
  - Error: net references non-existent component designator → hard error with net name + designator context
- **Note**: All proptest blocks must be gated with `#[cfg(feature = "proptest")]` per CLAUDE.md

**Code Intent**:
- New `sync.rs`: Define `SyncSnapshot`, `SyncComponent`, `SyncPin`, `SyncNet` structs. Implement `project_schdoc_spec()`: iterate sheets → collect components by designator → iterate nets and powers to fill pin.net. Implement `project_pcbdoc_spec()`: iterate boards → collect components → collect nets.
- `lib.rs`: Add `pub mod sync;` and re-exports.

**Code Changes**: _(Developer fills diffs)_

---

### Milestone 5: Diff Algorithm

**Files**:
- `crates/altium-format-spec/src/sync.rs`

**Flags**: `complex-algorithm`, `needs-rationale`

**Requirements**:
- `SyncChange` enum: `AddComponent`, `RemoveComponent`, `UpdateComponent`, `AddNet`, `RemoveNet`, `UpdateNet`, `AddPin`, `RemovePin`, `UpdatePin`
- `FieldChange` struct: field name, old_value (Option), new_value (Option)
- `diff_snapshots(source: &SyncSnapshot, target: &SyncSnapshot) -> Vec<SyncChange>`
- Diff is direction-agnostic: "what target must change to match source"
- Component matching by designator, net matching by name, pin matching by designator within matched components
- For matched components: compare comment, footprint, source_library, each parameter
- `SyncPolicy` struct with per-property `SyncDirection` (Forward, Back, Bidirectional, None)
- `filter_changes(changes: &[SyncChange], policy: &SyncPolicy, direction: SyncDirection) -> Result<Vec<SyncChange>, SpecError>` — returns `Err` with "pin-level sync not supported in Phase 1" if any AddPin/RemovePin/UpdatePin variant is encountered

**Acceptance Criteria**:
- Identical snapshots → empty changeset
- Source has component not in target → `AddComponent`
- Target has component not in source → `RemoveComponent`
- Same component, different footprint → `UpdateComponent` with `FieldChange { field: "footprint", ... }`
- Source has net not in target → `AddNet`
- Pin on source component has net, same pin on target has different net → `UpdatePin`
- `diff_snapshots(a, b)` followed by `diff_snapshots(b, a)` produces inverse changes
- Policy filtering: `SyncDirection::None` for a property suppresses all changes to that property in the filtered output (the `FieldChange` for that property is excluded from the result)
- Update STATUS.md to reflect diff algorithm implementation

**Tests**:
- **Test files**: `crates/altium-format-spec/src/sync.rs` (inline `#[cfg(test)]`)
- **Test type**: property-based (proptest)
- **Backing**: user-specified
- **Scenarios**:
  - Property: `diff(a, a) == []` (reflexive), `diff(a, b).len() > 0` when `a != b`
  - Property: `apply(diff(a, b), b)` produces snapshot where `diff(a, b') == []` (convergent)
  - Normal: add component, remove component, update footprint, add net, pin reassignment
  - Edge: empty snapshots, single-component diff, component with many parameters
- **Note**: All proptest blocks must be gated with `#[cfg(feature = "proptest")]` per CLAUDE.md

**Code Intent**:
- `sync.rs`: Add `SyncChange` enum and `FieldChange` struct. Implement `diff_snapshots()`: two passes — (1) iterate source components, match against target by designator → Add/Update/unchanged, then iterate target components not in source → Remove; (2) same for nets. Within matched components, diff each field and diff pins. Add `SyncPolicy` struct (do NOT derive or impl `Default` — an all-None policy silently skips all sync; see Decision Log), `SyncDirection` enum (do NOT derive `Default`), `filter_changes()`. `filter_changes()` MUST return `Err` if it encounters any `AddPin`/`RemovePin`/`UpdatePin` variant — never silently strip pin connectivity changes (see Decision Log "filter_changes must hard-error on pin variants").

**Code Changes**: _(Developer fills diffs)_

---

### Milestone 6: Change Application & CLI

**Files**:
- `crates/altium-format-spec/src/sync.rs`
- `crates/altium-cli/src/main.rs`

**Flags**: `error-handling`

**Requirements**:
- `apply_sync_changes_to_pcbdoc(changes: &[SyncChange], spec: &mut PcbDocSpec) -> Result<(), SpecError>`
  - Guard: if `spec.boards.is_empty()`, return `Err(SpecError::invalid_spec("PcbDoc spec must contain at least one board block for sync apply"))`
  - Guard: if `spec.boards.len() != 1`, return `Err(SpecError::not_supported("Multi-board PcbDoc specs are not supported by spec sync in Phase 1"))`
  - AddComponent: append `PcbDocComponentSpec` with designator, pattern, comment; location/rotation/layer = None
  - RemoveComponent: remove by designator
  - UpdateComponent: update matching fields (never touch location/rotation/layer)
  - AddNet: append `PcbDocNetSpec` with name; color = None (Altium uses system default)
  - RemoveNet: remove by name
  - AddPin/RemovePin/UpdatePin: diff MUST NOT generate these changes in Phase 1 (PcbDoc lacks pin connectivity). If somehow generated, apply returns hard error "pin-level sync not supported in Phase 1" — never silently drop connectivity changes
- Changes applied in dependency order: removes before adds, containers before members (3-phase ordering; see Decision Log "3-phase ordering suffices")
- All fallible operations use `.with_context(|| format!("applying sync change {:?} to PcbDoc spec", change))` per CLAUDE.md error context requirements
- CLI subcommand: `altium spec sync --forward <schdoc-spec> <pcbdoc-spec>`
  - Parse both specs, compile, project to SyncSnapshot, diff, apply, write back
  - `--dry-run`: print ECO report without writing
  - Output: ECO-style text report of changes applied
- CLI subcommand: `altium spec sync --diff <schdoc-spec> <pcbdoc-spec>` (diff only, no apply)
- Back-annotation apply (`apply_sync_changes_to_schdoc`) is NOT in Phase 1 scope — `--back` CLI flag deferred to Phase 2. `filter_changes` Back direction support is present as forward-compatible scaffolding only, not exercised in Phase 1

**Acceptance Criteria**:
- Forward sync adds missing components to PcbDoc spec
- Forward sync removes extra components from PcbDoc spec
- Forward sync updates footprint/comment on existing components
- Forward sync does NOT modify location/rotation/layer on existing components
- `--dry-run` prints changes but does not modify files
- Idempotent: running sync twice produces no changes on second run
- Error: missing input file → clear error message with file path
- Error: empty PcbDoc boards → clear error "must contain at least one board block"
- Error: multi-board PcbDoc → clear error "not supported in Phase 1"
- Update STATUS.md to reflect newly implemented capabilities: annotation syntax, sync IR, diff algorithm, forward sync CLI

**Tests**:
- **Test files**: `crates/altium-format-spec/src/sync.rs` (inline `#[cfg(test)]`), `crates/altium-cli/src/main.rs` (inline `#[cfg(test)]`)
- **Test type**: example-based integration (spec text → parse → compile → project → diff → apply → verify)
- **Backing**: user-specified
- **Scenarios**:
  - Normal: SchDoc adds component not in PcbDoc → forward sync adds it
  - Normal: SchDoc removes component → forward sync removes from PcbDoc
  - Normal: SchDoc changes footprint → forward sync updates PcbDoc
  - Edge: PcbDoc has component location → forward sync preserves it
  - Edge: empty SchDoc spec → forward sync removes all PcbDoc components
  - Idempotency: apply, then diff again → empty changeset

**Code Intent**:
- `sync.rs`: Add `apply_sync_changes_to_pcbdoc()`. Sort changes by type (Remove* first, then Update*, then Add*). For each change, find/modify/add/remove the corresponding entry in PcbDocSpec's boards[0].
- `main.rs`: Add `Spec` subcommand group with `Sync` subcommand. Parse `--forward`, `--diff`, `--dry-run` flags. Implement sync pipeline: read files → parse → compile → validate → project → diff → **filter_changes(changes, policy, SyncDirection::Forward)** → optionally apply → print report. Call `filter_changes` between diff and apply on every code path including `--dry-run` — it enforces SyncPolicy and hard-errors on pin variants. Construct `SyncPolicy` explicitly with named `SyncDirection` per property based on the `--forward`/`--back` flag; never call `SyncPolicy::default()` (see Decision Log "SyncPolicy is always explicitly constructed"). Phase 1 `--forward` property mapping (see Decision Log "Phase 1 forward SyncPolicy property mapping"): `comment: Forward, footprint: None, source_library: None, parameters: Forward, net_name: Forward, net_color: None, pin_net_assignment: None, component_location: None`. Use atomic write (tempfile + rename) when writing modified spec back to disk (see Decision Log "CLI spec write-back uses atomic temp+rename").

**Code Changes**: _(Developer fills diffs)_

---

### Milestone 7: Validator & Resolver (Multi-Phase Pipeline)

**Files**:
- `crates/altium-format-spec/src/validator.rs` _(new)_
- `crates/altium-format-spec/src/resolver.rs` _(new)_
- `crates/altium-format-spec/src/lib.rs`
- `crates/altium-cli/src/main.rs`

**Flags**: `error-handling`

**Requirements**:
- `validator.rs`: Phase 3 consistency checks on compiled SpecModel
  - Duplicate designators across sheets → error
  - Net references to non-existent component designators → error
  - Duplicate annotation IDs → error (authoritative check; compiler also has fast-fail duplicate detection within single files — see Decision Log "Duplicate ID detection is dual-layer")
  - Pin references to non-existent pins → `SpecError` with `Severity::Warning` (pins may come from library, not yet resolved; callers filter by severity — see Decision Log "SpecError gains Severity field")
- `resolver.rs`: Phase 4 library resolution
  - `ResolvedSpec` struct: `footprint_map: HashMap<String, String>` (designator → footprint name)
  - `resolve_spec(model: &SpecModel, libraries: &[SchLibSpec]) -> Result<ResolvedSpec>`
  - For each SchDoc component with a symbol reference, look up the library to find footprint mappings
  - If library is referenced but not available, return hard error: "cannot resolve library 'X' referenced by component 'Y'" — no silent degradation
  - If component has no library reference (bare designator), footprint remains None (this is valid — not all components have library refs)
- Both `validate_schdoc_spec` and `validate_pcbdoc_spec` return `Result<Vec<SpecError>, Vec<SpecError>>` — `Ok(warnings)` for success-with-warnings, `Err(errors)` for hard failures

**Acceptance Criteria**:
- Validator catches duplicate designators and produces error with both locations
- Validator catches net referencing unknown component and produces error with net name and component designator
- Resolver populates footprint map from SchLib when available
- Resolver returns hard error when a referenced library is unavailable (fail fast — no silent footprint loss)
- Resolver returns empty footprint map only when no library references exist (valid case)
- CLI `spec sync` calls validator before diff (fail-fast on invalid specs)
- Update STATUS.md to reflect validator and resolver capabilities

**Tests**:
- **Test files**: `crates/altium-format-spec/src/validator.rs`, `crates/altium-format-spec/src/resolver.rs` (inline `#[cfg(test)]`)
- **Test type**: example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: valid spec passes validation with no errors
  - Error: duplicate designator, dangling net reference, duplicate annotation ID
  - Normal: resolver with library → footprint map populated
  - Edge: resolver without library → empty map, no error

**Code Intent**:
- New `validator.rs`: `validate_schdoc_spec(spec: &SchDocSpec) -> Result<Vec<SpecError>, Vec<SpecError>>` and `validate_pcbdoc_spec(spec: &PcbDocSpec) -> Result<Vec<SpecError>, Vec<SpecError>>`. Check designator uniqueness, net reference validity, annotation ID uniqueness. Returns `Result<Vec<SpecError>, Vec<SpecError>>`: `Ok(warnings)` carries non-fatal warnings (e.g., unresolved pin refs), `Err(errors)` carries hard errors. CLI prints `Ok(warnings)` to stderr before proceeding; converts `Err(errors)` to single `anyhow::Error` (see Decision Log "Validator return type carries warnings on success path").
- New `resolver.rs`: `ResolvedSpec` struct. `resolve_schdoc_spec(model: &SchDocSpec, libraries: &[SchLibSpec]) -> Result<ResolvedSpec>`. Iterate components, look up symbol in libraries, extract footprint mappings.
- `lib.rs`: Add `pub mod validator; pub mod resolver;`.
- `main.rs`: In the `spec sync` CLI handler, after the compile step and before calling `project_schdoc_spec()`/`project_pcbdoc_spec()`, call `validate_schdoc_spec()`/`validate_pcbdoc_spec()`. On `Ok(warnings)`, print each warning to stderr. On `Err(errors)`, join errors into a single `anyhow::Error` with `.with_context()` at the Vec→anyhow boundary and return early (see Decision Log "Validator return type carries warnings on success path").

**Code Changes**: _(Developer fills diffs)_

---

### Milestone 8: Constraint & Rule Extensions

**Files**:
- `crates/altium-format-spec/src/ast.rs`
- `crates/altium-format-spec/src/parser.rs`
- `crates/altium-format-spec/src/compiler.rs`
- `crates/altium-format-spec/src/model.rs`

**Flags**: `conformance`

**Requirements**:
- SchDoc `SheetSpec` gains `constraints: Vec<ConstraintSpec>` for schematic-domain constraints
- `ConstraintSpec` struct: `kind: ConstraintKind` (typed enum — see Decision Log), `properties: IndexMap<String, String>`, `annotation: Option<CompiledAnnotation>`
- PcbDoc `PcbDocRuleSpec` expanded: `properties: IndexMap<String, String>`, `scope: Option<String>`, `annotation: Option<CompiledAnnotation>`
- Parser handles `constraint <kind> { key: value, ... }` blocks within sheets
- Parser handles expanded `rule <name> { kind: "...", gap: ..., scope: "...", properties { ... } }` blocks within boards
- Annotations can be placed on constraint and rule blocks
- Constraints and rules participate in SyncSnapshot (projected to `SyncSnapshot.rules` map, future)

**Acceptance Criteria**:
- `constraint <kind> { min_trace_width: 5mil }` parses within a sheet block (specific `<kind>` value to be user-confirmed before implementation; see Decision Log "ConstraintKind initial variants")
- `rule r_clearance { kind: "clearance", gap: 5mil, scope: "all_copper" }` parses within a board block
- Annotations on constraints: `#[annotation(id = "CONS01")] constraint ...` works
- Rules compile to expanded `PcbDocRuleSpec` with properties map
- Existing specs without constraints continue to parse and compile
- Update STATUS.md to reflect constraint and rule extension capabilities

**Tests**:
- **Test files**: `crates/altium-format-spec/src/parser.rs` (inline `#[cfg(test)]`)
- **Test type**: example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: constraint block with properties, rule block with expanded fields
  - Edge: empty constraint block, rule with only name and kind
  - Error: constraint outside sheet block, unknown constraint kind (typo in ConstraintKind enum), unknown keyword in rule block body

**Code Intent**:
- BLOCKED: Do not implement `ConstraintKind` enum variants until user confirms exact variant names (see Decision Log "ConstraintKind initial variants"). The parser infrastructure can be built with a placeholder `ConstraintKind` that only accepts a single sentinel variant for testing.
- `ast.rs`: Add `ConstraintDecl` struct (kind, body, annotation). Add `Constraint(ConstraintDecl)` variant to `SheetItem`.
- `parser.rs`: Add `parse_constraint_decl()`. Update `parse_sheet_item()` to recognize `constraint` keyword. Update `parse_rule_decl()` to handle `scope` and `properties { }` sub-block.
- `model.rs`: Add `ConstraintSpec` struct. Add `constraints: Vec<ConstraintSpec>` to `SheetSpec`. Expand `PcbDocRuleSpec` with `properties: IndexMap<String, String>`, `scope: Option<String>`.
- `compiler.rs`: Add `compile_constraint_decl()`. Update `compile_rule_decl()` for expanded fields.

**Code Changes**: _(Developer fills diffs)_

---

### Milestone 9: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `crates/altium-format-spec/CLAUDE.md` (index updates)
- `crates/altium-format-spec/README.md` (invisible knowledge)
- `docs/doc-sync/sync-ir-design.md` (update with implementation details)

**Requirements**:

Delegate to Technical Writer. Deliverables:
- CLAUDE.md: Add entries for new modules (annotation.rs, sync.rs, validator.rs, resolver.rs)
- README.md: Architecture diagram, data flow, invariants, tradeoffs from Invisible Knowledge
- sync-ir-design.md: Update with actual struct names, module locations, CLI usage examples

**Acceptance Criteria**:
- CLAUDE.md is tabular index only (no prose sections)
- README.md is self-contained (no external references)
- Architecture diagram in README.md matches plan's Invisible Knowledge section

## Milestone Dependencies

```
M1 (Annotation Syntax)
 │
 ▼
M2 (Annotation Compile + ID Gen)
 │
 ├──▶ M3 (Dump & Formatter)
 │
 ▼
M4 (SyncSnapshot & Projection)
 │
 ▼
M5 (Diff Algorithm)
 │
 ▼
M6 (Change Application & CLI)
 │
 ├──▶ M7 (Validator & Resolver) ──── can start after M2
 │
 ├──▶ M8 (Constraint & Rule Extensions) ──── can start after M2
 │
 ▼
M9 (Documentation) ──── after all implementation milestones
```

**Parallel opportunities**:
- M3 (Dump) can run in parallel with M4-M6 (sync pipeline)
- M7 (Validator/Resolver) can start after M2, run in parallel with M4-M6
- M8 (Constraint/Rule extensions) can start after M2, run in parallel with M4-M6
