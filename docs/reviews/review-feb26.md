# Consolidated Codebase Review: altium-cli

*Synthesized from 5 independent reviews, February 2026*

## Executive Summary

The workspace has a strong crate-level architecture: `altium-format-derive` (proc macros) → `altium-format` (domain + parsing) → `altium-cli` (interface). The project uses good ecosystem choices (`clap`, `serde`, `thiserror`), has strong low-level building blocks (`Coord`, `Layer`, record enums), and covers an impressive breadth of Altium artifacts.

The dominant systemic problem is **boundary erosion within crates**. The ops modules in `altium-format` and command modules in `altium-cli` have grown into monolithic hybrids that blend file I/O, domain transformation, output formatting, and CLI orchestration. This makes the codebase hard to test, hard to evolve, and prone to duplicated logic that drifts. Every review independently identified this as the central structural issue.

Secondary concerns are **type discipline inconsistency** (strong domain types coexist with stringly-typed CLI surfaces and `Box<dyn Error>` everywhere) and **several correctness bugs** where known types are silently degraded, data is silently dropped, or parse failures are swallowed.

## Recommended Approach

**Phase 1 — Refactor ops + CLI for testability** (P1). This is the foundational change. Extract domain services, thin the CLI command layer, and consolidate duplicated logic. This makes everything else safer to tackle.

**In parallel — Fix critical data integrity bugs** (P2). Don't block these on the refactoring. The polygon roundtrip and `remove_param` bugs can produce silently corrupt output; they should be fixed immediately.

**Phase 2 — Type discipline** (P3). Replace stringly-typed interfaces with enums and standardize error handling. These naturally follow the service extraction in Phase 1.

**Phase 3 — Behavior correctness, performance, hygiene** (P4–P5). Query semantics fixes, clone reduction, API surface tightening.

---

## P1: Ops Module + CLI Testability Refactoring

*The foundational change. Flagged by all 5 reviews.*

### 1a. Extract domain services from ops god modules
- **Severity**: warning (structural, high change-cost impact)
- **Location(s)**: `crates/altium-format/src/ops/schlib.rs` (~3500 LOC), `crates/altium-format/src/ops/pcblib.rs` (~3475 LOC), `crates/altium-format/src/ops/pcbdoc.rs` (~2688 LOC)
- **Problem**: Ops modules combine I/O (file open/save), domain transforms (record manipulation, filtering, validation), and presentation-oriented payload assembly (`*Json` struct building, formatting). A top-of-file comment in `pcblib.rs` acknowledges this debt. The result is that command functions cannot be unit-tested without actual files and cannot be reused outside the CLI context.
- **Evidence**:
  - `ops/pcbdoc.rs` contains both parsing helpers (`parse_coord`) and high-level command orchestration
  - `ops/schlib.rs` documents that cmd functions mix presentation and business logic
  - `ops/pcblib.rs` imports parsing/rendering/output concerns simultaneously
- **Recommendation**: Split each ops module into:
  - **Service layer**: pure functions taking typed inputs → typed outputs (no I/O, no formatting)
  - **I/O adapter**: file open/save, stream handling
  - **Presentation adapter**: JSON struct building, output shaping
  - Start with the most-changed module to get immediate velocity benefits.

### 1b. Thin CLI command adapters
- **Severity**: warning (structural)
- **Location(s)**: `crates/altium-cli/src/commands/pcbdoc.rs` (~1191 LOC), `crates/altium-cli/src/commands/edit.rs` (~829 LOC), `crates/altium-cli/src/commands/pcblib.rs` (~799 LOC), `crates/altium-cli/src/commands/schdoc.rs` (~734 LOC), `crates/altium-cli/src/commands/query.rs`
- **Problem**: Command modules perform argument parsing, file I/O, domain logic, aggregation, timing, and rendering in single flows. `query.rs` opens files, constructs trees, executes queries, builds output DTOs, and prints in one function. `edit::run` calls `std::process::exit(1)` on failure.
- **Evidence**:
  - `pcbdoc.rs::run` is a very large dispatch function with many branches and repeated `output::print` plumbing
  - `query.rs:117-195` performs the entire pipeline inline
  - `edit::run` parses operation, executes edits, prints output, then exits process on failure
- **Recommendation**: Command modules should become thin adapters: parse args → call service → format output → return result. No `process::exit` outside `main`. No file I/O inside business logic.

### 1c. Consolidate duplicated mappings into shared services
- **Severity**: warning (DRY violation)
- **Location(s)**:
  - `crates/altium-format/src/ops/util.rs::record_type_name` and `crates/altium-format/src/ops/schlib.rs::record_type_name` — nearly identical `SchRecord → &str` mapping
  - `crates/altium-cli/src/commands/edit.rs` and `crates/altium-format/src/ops/schdoc_edit.rs` — same power style/orientation/port-I/O parsing with same error messages
- **Problem**: Duplicated mapping tables and parsing logic guarantee drift when one copy is updated but not the other.
- **Recommendation**: Centralize in single source-of-truth modules. This naturally falls out of the service extraction in 1a — shared types and mappings belong in the service layer.

### 1d. Typed return values from services
- **Severity**: warning (testability enabler)
- **Problem**: Current ops functions return presentation-ready payloads (JSON structs, formatted strings), making it impossible to assert on domain semantics in tests. Services should return domain types; presentation adapters convert to output formats.
- **Recommendation**: Define typed result structs per operation family. Services return these; CLI adapters convert to `Serialize`-able output models.

---

## P2: Data Integrity Bugs

*Silent data loss or corruption. Fix in parallel with P1.*

### 2a. Polygon roundtrip degrades known type into Unknown (3/5 reviews)
- **Severity**: critical
- **Location(s)**: `crates/altium-format/src/ops/pcblib.rs:3127-3130`, `crates/altium-format/src/ops/pcblib.rs:2890-2904`, `crates/altium-format/src/ops/pcblib.rs:3378-3386`
- **Problem**: `PcbRecord::Polygon` is explicitly mapped to `PcbPrimitiveJson::Unknown` with `raw_data: String::new()`. Known typed data is forced into an unknown channel with empty payload — data is lost on roundtrip.
- **Evidence**: `PcbRecord::Polygon(_) => PcbPrimitiveJson::Unknown { ... raw_data: String::new() }`
- **Recommendation**: Add a first-class `Polygon` variant to `PcbPrimitiveJson` with bidirectional conversion. Add roundtrip tests asserting polygon fidelity through JSON conversion.

### 2b. `UnknownFields::remove_param` returns success without removing data (1/5 reviews)
- **Severity**: critical
- **Location(s)**: `crates/altium-format/src/types/unknown.rs:157-166`
- **Problem**: Method name and return value imply removal, but only `param_order` is updated — the `ParameterCollection` entry remains. Stale data is re-emitted during roundtrip serialization.
- **Evidence**: `self.param_order.retain(|k| k != key);` followed by `return true`, with comment: *"ParameterCollection doesn't have remove... For now, we just remove from order tracking"*.
- **Recommendation**: Add `remove` support to `ParameterCollection` and truly remove from both containers. Add regression test.

### 2c. Silent component truncation in PcbDoc reader (1/5 reviews)
- **Severity**: critical
- **Location(s)**: `crates/altium-format/src/io/pcbdoc.rs` (~lines 157-163)
- **Problem**: Component iteration swallows parse errors and breaks the loop. A single malformed component causes all subsequent components to be silently dropped, returning a partial document without error.
- **Evidence**: `while ... { match self.read_component_record(...) { Ok(comp) => push, Err(_) => break } }`
- **Recommendation**: Treat parse failure as structured error with context (offset/index). Either collect recoverable errors and continue, or fail with diagnostics. Never silently truncate.

### 2d. Silent base64 decode fallback masks corruption (2/5 reviews)
- **Severity**: critical
- **Location(s)**: `crates/altium-format/src/ops/pcblib.rs:3383-3386`
- **Problem**: Malformed base64 in JSON payloads is silently converted to empty bytes via `unwrap_or_default()`, hiding data corruption.
- **Evidence**: `decode(raw_data).unwrap_or_default()`
- **Recommendation**: Propagate decode failures as typed errors with primitive index context.

### 2e. ComponentBody parsed as PcbRegion (1/5 reviews)
- **Severity**: warning (currently correct but fragile)
- **Location(s)**: `crates/altium-format/src/v2/pcb/io/pcblib.rs`
- **Problem**: `ComponentBody` objects are stored in `Vec<PcbRegion>` and parsed with `PcbRegion::read_from` even though the object ID is explicitly `PcbObjectId::ComponentBody`. This works because binary layouts happen to match, but ties correctness to an implicit assumption.
- **Recommendation**: Introduce a dedicated `PcbComponentBody` type (or newtype wrapper) with explicit conversions.

### 2f. PcbLibFootprint mixes raw and typed representations (1/5 reviews)
- **Severity**: warning (invalid states possible)
- **Location(s)**: `crates/altium-format/src/v2/pcb/io/pcblib.rs`
- **Problem**: `PcbLibFootprint` contains both typed vectors (`tracks`, `pads`) and raw bytes (`raw_primitives`, `raw_parameters`) simultaneously. `write()` resolves conflicts with fallback logic rather than type invariants.
- **Evidence**: One struct has both typed and raw representations; `write()` recomputes count from either `primitive_order` or summed vectors.
- **Recommendation**: Split into explicit states (`RawFootprint` vs `TypedFootprint`) or enforce invariants via builder/conversion step.

---

## P3: Type Discipline

*Compile-time safety improvements. Best tackled after P1 service extraction.*

### 3a. Stringly-typed CLI interfaces → enums (5/5 reviews)
- **Severity**: warning
- **Location(s)**:
  - Output format: `crates/altium-cli/src/main.rs:153-157`, `crates/altium-cli/src/output.rs:18-37`
  - Edit operations: `crates/altium-cli/src/commands/edit.rs` — `EditOperation::AddPower { style: String, orientation: String }`, `AddPort { io_type: String }`
  - Shell completions: `Completions { shell: String }`
  - Mask expansion: `crates/altium-format/src/ops/pcblib.rs:2908-2913` — `PcbMaskExpansionJson { mode: String }`
- **Problem**: Output modes (`"text"`, `"json"`, `"json-pretty"`), edit parameters, shell names, and mask modes are passed as raw strings and matched with runtime string comparisons. Invalid values travel deep before failing.
- **Recommendation**: Introduce enums (`OutputFormat`, `PowerStyle`, `Orientation`, `PortIoType`, `ShellKind`, `MaskExpansionMode`) with `clap::ValueEnum` or `FromStr`. Parse once at CLI boundary.

### 3b. Error type soup → typed error enums (4/5 reviews)
- **Severity**: warning
- **Location(s)**: `crates/altium-cli/src/main.rs:150`, all `commands/*.rs` `run` signatures, many `crates/altium-format/src/ops/*.rs` command functions
- **Problem**: The codebase defines a rich `AltiumError` (thiserror), but most public command paths return `Result<_, Box<dyn std::error::Error>>`. This erases error taxonomy, blocks differentiated handling, and makes mapping to CLI exit codes brittle.
- **Recommendation**: Standardize on layered typed errors:
  1. `altium-format`: `AltiumError` + sub-enums per ops domain
  2. `altium-cli`: `CliError` enum with `From` conversions from domain errors
  3. Only use `Box<dyn Error>` at the `main` boundary if necessary

---

## P4: Behavior Correctness

*Wrong results (not data loss). Lower urgency but still important.*

### 4a. Lossy wildcard operator mapping in query engine (1/5 reviews)
- **Severity**: critical (for query accuracy)
- **Location(s)**: `crates/altium-format/src/query/selector.rs` (~lines 348-354), `crates/altium-format/src/query/common.rs` (~lines 166-179)
- **Problem**: `FilterOperator::Wildcard` is converted to `FilterOp::WordMatch` ("closest equivalent"), but `WordMatch` only performs whitespace token equality. Wildcard/pattern behavior is semantically different, causing incorrect query results.
- **Evidence**: `Self::Wildcard => FilterOp::WordMatch // Closest equivalent`; `WordMatch` splits on whitespace and compares plain strings.
- **Recommendation**: Add explicit `PatternMatch` semantics to `FilterOp`. Unify the filter operator/value model instead of coercing through lossy conversions.

### 4b. Query language detection is heuristic-based (1/5 reviews)
- **Severity**: warning
- **Location(s)**: `crates/altium-cli/src/commands/query.rs:198-225`
- **Problem**: `is_schql_query` infers language via substring heuristics (`contains(':')`, `contains('>')`, etc.). This can misclassify edge inputs.
- **Recommendation**: Attempt parse with one grammar and fall back based on structured parse errors, or require explicit `--lang` flag with optional auto mode.

---

## P5: Performance and Hygiene

*Lower priority. Address opportunistically or as part of larger refactors.*

### 5a. Clone-heavy query paths (3/5 reviews)
- **Severity**: warning
- **Location(s)**: `crates/altium-cli/src/commands/query.rs:132,171`
- **Problem**: `RecordTree::from_records(doc.primitives.clone())` and `RecordTree::from_records(component.primitives.clone())` — full collection clone before every tree build.
- **Recommendation**: Add borrowed tree construction (`from_records_ref`) or refactor query engine to consume iterators/borrowed slices.

### 5b. Numeric cast safety in binary I/O (2/5 reviews)
- **Severity**: note
- **Location(s)**: `crates/altium-format/src/records/pcb/outline.rs`, `crates/altium-format/src/v2/pcb/pad.rs`, `crates/altium-format/src/v2/serializer/binary.rs`
- **Problem**: `f64 → i32` via `as`, `try_into().unwrap()` in non-test code, `partial_cmp(...).unwrap()` (NaN can panic).
- **Recommendation**: Add checked conversion helpers with explicit error paths. Use `total_cmp` or NaN filtering for float ordering.

### 5c. Serializer TODOs in production paths (3/5 reviews)
- **Severity**: note
- **Location(s)**: `crates/altium-format/src/v2/serializer/ascii.rs:545-548`, `crates/altium-format/src/v2/serializer/binary.rs:183-185`
- **Problem**: TODO stubs for `hex+zlib` encoding and `Real48` angle handling — known behavior gaps in core format infrastructure.
- **Recommendation**: Track with issues and test coverage gates. Consider hard errors instead of silent incomplete behavior.

### 5d. Leaky public API surface (2/5 reviews)
- **Severity**: note
- **Location(s)**: `crates/altium-format/src/lib.rs:91-122`, `crates/altium-format/src/v2/fields/primitives.rs`, `crates/altium-format/src/v2/fields/component.rs`
- **Problem**: Most internal modules are publicly exported. Record structs expose all fields as mutable `pub`, including low-level serialization details. `ComponentData` has 47+ public fields spanning identity, graphical state, vault metadata, and rendering flags.
- **Recommendation**: Narrow surface to stable entry points. Restrict to `pub(crate)` where possible. Split `ComponentData` into cohesive sub-structs with validated constructors.

### 5e. Cache accessor duplication (1/5 reviews)
- **Severity**: note
- **Location(s)**: `crates/altium-format/src/api/document.rs:65-105`
- **Problem**: Four methods duplicate load/insert/get logic with `contains_key` + `get(...).unwrap()` double-lookup pattern.
- **Recommendation**: Refactor via `entry` API to eliminate duplication and unwraps.

### 5f. Collect-then-iterate patterns (1/5 reviews)
- **Severity**: note
- **Location(s)**: `crates/altium-format/src/ops/pcbdoc.rs` — `cmd_rules` and similar list operations
- **Problem**: Intermediate vectors collected then immediately iterated/mapped again.
- **Recommendation**: Streamline iterator chains directly into final targets unless intermediate materialization is needed for sorting/reuse.

---

## Systemic Patterns

These patterns appear across most or all reviews and represent the underlying themes:

1. **Boundary erosion** (5/5 reviews): I/O, domain logic, and presentation are repeatedly mixed in the same modules and functions, both in `altium-format::ops` and `altium-cli::commands`.

2. **Type discipline inconsistency** (5/5 reviews): Strong domain types exist internally (`Coord`, `Layer`, record enums, `AltiumError`) but CLI/ops boundaries collapse to strings and `Box<dyn Error>`.

3. **Repetition under scale** (4/5 reviews): Each new command path copies orchestration patterns rather than extending shared typed pipelines. Mapping tables are duplicated across modules.

4. **Roundtrip fidelity risk** (3/5 reviews): Known types are treated as unknown, raw and typed representations coexist in the same structs, and silent fallbacks mask corruption.

5. **Performance headwinds** (3/5 reviews): Clone-heavy paths, collect-then-iterate, and broad allocations will scale poorly as file sizes grow.

## Architecture Assessment

**What is working well:**
- Crate-level layering is clean and intentional (derive → format → cli)
- Typed `AltiumError` in core, `thiserror` usage, coordinate wrappers
- Explicit binary format behavior documentation (valuable for reverse-engineered formats)
- Breadth of supported Altium artifact types

**What needs attention:**
- Intra-crate layering: `ops` acts as a monolithic application layer, not a thin service facade
- CLI commands are mini-applications rather than thin adapters
- The type model is undermined at boundaries where user-facing correctness matters most
- Dependency direction is healthy at crate level but responsibility direction is tangled within crates

**Overall:** Good macro-architecture, weak micro-architecture in the application/service layers. The refactoring in P1 is the single highest-leverage improvement — it enables testability, reduces change cost, and makes all subsequent fixes safer.
