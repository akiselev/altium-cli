# FCIS v2: Clean-Slate Architecture for v1/v2 APIs

## Intent

Design from scratch for one goal: **lossless, nondestructive editing with an ergonomic API**, while keeping a strict **Functional Core / Imperative Shell** split.

This document intentionally ignores migration constraints. Existing code is treated as a knowledge base and oracle; the new architecture is allowed to break old APIs.

## Non-Negotiables

1. **Lossless by default**: if nothing is edited, output bytes are byte-identical for every untouched stream.
2. **Patch, do not rebuild**: editing a record patches original representation; unknown/order/trivia data is preserved.
3. **No core defaults**: contextual defaults live in DTO/interface layers only (CLI/JSON/UI), never in core record types.
4. **No `pub` fields**: generated record fields are private; API is getters/setters/mutators.
5. **Macro-first generation**: reuse v1 macro idea, but generate against v2 editing semantics.
6. **Tests are behavioral**: keep current fixtures/tests, but assertions must prove functional behavior, not just counts.

## FCIS Boundary

### Functional Core

Pure, deterministic, in-memory:

- Parse raw streams into origin-preserving structures.
- Project raw records into typed entities.
- Apply typed edits through generated mutators.
- Produce patch plans from `(origin, dirty fields)`.
- Materialize patched bytes deterministically.

Core must not do:

- Filesystem or CFB/OLE I/O.
- CLI printing/logging.
- Context-based defaulting.

### Imperative Shell

Owns side effects and orchestration:

- Open/write OLE/CFB containers.
- Locate streams and select document backend.
- Apply context defaults at DTO boundaries.
- Convert core errors to user-facing errors.

## Clean-Slate Model

### Core Types

```rust
pub struct DocumentCore {
    streams: Vec<StreamNode>,
}

pub struct StreamNode {
    id: StreamId,
    origin: StreamOrigin,
    records: Vec<RecordNode>,
    dirty: bool,
}

pub struct RecordNode {
    key: RecordKey,
    origin: RecordOrigin,
    entity: EntityBox,
    dirty_fields: DirtyFieldSet,
}
```

```rust
pub enum RecordOrigin {
    Param(ParamOrigin),
    Binary(BinaryOrigin),
}

pub struct ParamOrigin {
    full_pairs: Vec<ParamPair>,   // full original order/casing/raw token text
    index: ParamIndex,            // fast lookup
    raw_record_text: String,      // original serialized record body
}

pub struct BinaryOrigin {
    raw_block: Vec<u8>,           // original binary record bytes
    field_spans: Vec<FieldSpan>,  // decoded span map for patching
}
```

`UnknownFields` is replaced by `RecordOrigin` plus patch planning. Unknown data is not a side bucket; it is part of the canonical origin.

## API Shape (Ergonomic Outside, Explicit Inside)

### Typed Entity API

Generated entities expose private state and explicit accessors:

```rust
impl SchPin {
    pub fn designator(&self) -> FieldRef<'_, Designator>;
    pub fn try_designator(&self) -> Result<&Designator, MissingField>;

    pub fn set_designator(&mut self, v: impl Into<Designator>) -> Result<(), EditError>;
    pub fn update_designator<F>(&mut self, f: F) -> Result<(), EditError>
    where
        F: FnOnce(&Designator) -> Designator;
}
```

Getter/mutator surface supports both strict and ergonomic usage:

- `try_*` APIs for core callers that need explicit missing/error handling.
- `*()` convenience getters for consumers that accept optional semantics.
- No implicit defaults are injected at entity level.

### DTO Boundary

Defaults are applied only in boundary mappers:

```rust
impl From<&SchPin> for SchPinDto {
    fn from(pin: &SchPin) -> Self {
        Self {
            // context default here, not in SchPin
            electrical: pin.try_electrical().unwrap_or(PinElectrical::Passive),
        }
    }
}
```

## Macro v3 Design

## Goals

1. Generate private fields and accessors/mutators.
2. Generate parse/project + patch/write code using `RecordOrigin`.
3. Remove `default` support from core field attributes.
4. Support configurable getter/setter behavior (validation, conversion, normalization).

## Example Declaration

```rust
#[derive(AltiumEntity)]
#[altium(kind = "sch", record_id = 2, codec = "params")]
struct SchPin {
    #[altium(param = "DESIGNATOR", get = "as_str", set = "normalize_designator")]
    designator: Designator,

    #[altium(param = "PINLENGTH", frac = "PINLENGTH_FRAC", set = "validate_pin_length")]
    pin_length: Coord,

    #[altium(param = "ELECTRICAL")]
    electrical: PinElectrical,
}
```

## Generated Pieces

- Entity struct with private fields.
- Getter/mutator methods.
- `ProjectFromOrigin` implementation.
- `PatchIntoOrigin` implementation.
- Dirty-field tracking.
- Optional generated `Builder` for creation paths.

### Getter/Setter Customization Hooks

Supported hook points (macro args):

- `get = "fn_name"`: map internal representation for reads.
- `set = "fn_name"`: validate/convert on write.
- `on_change = "fn_name"`: post-update invariants.
- `clear = true`: allow explicit unset for optional fields.

This keeps core strict while giving ergonomic call sites.

## Patch Engine (Core)

### Patch Planning

For each dirty record:

1. Compare current typed field values against original decoded values.
2. Emit field-level patch ops.
3. Apply ops against `RecordOrigin` preserving unknown/order/trivia.

```rust
pub enum PatchOp {
    ParamSet { key: String, value: String },
    ParamRemove { key: String },
    BinaryReplaceSpan { offset: usize, len: usize, bytes: Vec<u8> },
}
```

### Write Rules

- Untouched stream: write original bytes exactly.
- Touched stream: re-emit only modified records; preserve untouched records byte-for-byte.
- Deterministic ordering and encoding for inserted keys/records.

## Architecture Options for Entity Editing

### Option A: Stateful Entity with Embedded Origin (Recommended)

Shape:

- Each entity stores typed fields + reference/handle to origin + dirty mask.
- Setters mutate entity and mark dirty fields.
- Save compiles patches from entity state.

Pros:

- Most ergonomic user API.
- Efficient dirty tracking.
- Straight path from setter to patch.

Cons:

- Entity lifetime/ownership model is more complex.

### Option B: Immutable Entity + External PatchBuilder

Shape:

- Entity is immutable typed snapshot.
- Mutations create patch commands in separate builder.

Pros:

- Very pure functional model.
- Easier reasoning in concurrent contexts.

Cons:

- Heavier API for common edits.
- Easy for callers to desync snapshot and patch intent.

### Option C: Command-Only Delta API (No Setters)

Shape:

- Public API exposes `apply(Command)` only.

Pros:

- Maximum control and auditability.
- Good for bulk scripted edits.

Cons:

- Lowest ergonomics.
- Harder discoverability and IDE guidance.

### Decision

Choose **Option A** for primary API, with optional command export for batch workflows.

## v1/v2 Unification Strategy (Clean-Slate)

- One `DocumentCore` and one patch engine.
- Format-specific codecs implement the same core traits:
  - `DecodeOrigin`
  - `ProjectEntity`
  - `EncodePatch`
- v1 and v2 become codec implementations, not separate editing stacks.

## Test Strategy: Reuse Existing Tests, Raise Signal

Keep all existing test files under `crates/altium-format/tests`, but rewrite assertions around behavior.

## What to Keep from Current Tests

- Existing real-world fixtures and corpus files.
- Existing roundtrip entrypoints (`SchDoc`, `PcbDoc`, `SchLib`, `PcbLib`).
- Existing type-equality checks where they are meaningful.

## What to Change

1. Remove silent pass/skip patterns for required fixtures.
2. Replace hardcoded external absolute paths with fixture registry/harness.
3. Replace count-only checks with invariant checks.
4. Add targeted mutation assertions (one field changed, unrelated bytes unchanged).

## Required Test Categories

### 1) No-Edit Identity Tests

For every fixture:

- `open -> save` produces byte-identical output for untouched streams.
- Assert exact stream-level byte equality.

### 2) Single-Field Patch Tests

For each major record family:

- Mutate one typed field via generated setter.
- Assert patch modifies only expected param/span.
- Assert unknown/original ordering remains unchanged.

### 3) Multi-Field + Invariant Tests

- Mutate coupled fields and assert validators/converters ran.
- Assert generated patch is deterministic.

### 4) DTO Defaulting Tests

- Core entity missing field stays missing.
- DTO mapper applies context default.
- Roundtrip back into core does not fabricate hidden defaults.

### 5) Shell Contract Tests

- File open/save errors and path behavior.
- No business logic assertions here.

## Mapping Existing Tests to High-Signal Roles

- `crates/altium-format/tests/v2_schdoc_cfb_roundtrip.rs`
  - keep corpus coverage; add no-edit byte identity and one-field mutation assertions.
- `crates/altium-format/tests/v2_pcbdoc_cfb_roundtrip.rs`
  - keep deep typed comparisons; add patch-locality assertions.
- `crates/altium-format/tests/v2_schlib_cfb_roundtrip.rs`
  - keep structural checks; add unknown/order preservation checks.
- `crates/altium-format/tests/v2_pcblib_cfb_roundtrip.rs`
  - keep primitive equality checks; add span-level binary patch assertions.
- `crates/altium-format/tests/v2_schlib_roundtrip.rs`
  - repurpose from JSON-only equality to DTO-boundary/defaulting tests.
- `crates/altium-format/tests/v2_pcblib_roundtrip.rs`
  - repurpose similarly; keep typed deep checks where DTO semantics are involved.

## CLI Surface Plan (Query + High-Level Commands)

Do not remove commands in code right now. Track them in the FCIS plan with explicit freeze/remove/rebuild stages.

### Stage 1: Freeze During Core Refactor

- Freeze feature work on higher-level CLI flows, especially:
  - `crates/altium-cli/src/commands/query.rs`
  - orchestration-heavy commands that mix policy/business rules with output shaping.
- Allow only break/fix changes needed to keep build and baseline tests green.
- Do not introduce new query semantics while core patching/editing APIs are being rebuilt.

### Stage 2: Validate Core First

- Complete refactoring, test upgrades, and validation gates first:
  - no-edit identity guarantees
  - patch-locality guarantees
  - DTO-default boundary guarantees
- Keep old high-level commands as temporary wrappers only while these gates are being proven.

### Stage 3: Rip Out and Redesign

- After Stage 2 is green, remove query/high-level command implementations and replace with thin adapters over the new FCIS core.
- Reintroduce redesigned query/high-level features only after they are expressed as pure core operations plus shell adapters.
- New command APIs must not bypass core patch engine or reintroduce mixed FC/IS logic.

## Anti-Pointless Test Rules

A test is rejected unless it asserts at least one of:

1. Byte/stream identity of untouched data.
2. Exact locality of a patch after mutation.
3. Explicit invariant/validation behavior.
4. Explicit FC/IS boundary contract.

## Definition of Done

1. Core can edit v1/v2 records without any filesystem dependencies.
2. All generated entities use private fields and generated accessors/mutators.
3. Core has zero implicit defaults.
4. `UnknownFields` is removed from editing path in favor of full-origin patching.
5. Existing v2 test files still exist but now assert functional behavior with high signal.
6. Query/high-level CLI command redesign starts only after FCIS core/test/validation gates are complete.
