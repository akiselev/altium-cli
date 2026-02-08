# Codebase Review: altium-cli

## Executive Summary
This workspace has strong momentum and breadth: it covers binary format parsing, editing operations, and a full CLI for multiple Altium document types. The project already uses `thiserror`, typed coordinate wrappers (`Coord`, `PcbCoord`), and structured command modules, which are all good foundations.

The biggest systemic issue is boundary collapse. The codebase repeatedly mixes domain transformation, file I/O, CLI orchestration, and output formatting in the same oversized modules and functions. This creates high change-surface: even small behavior updates require touching large files, and duplicated patterns drift over time.

The second major issue is type discipline inconsistency. Some parts are strongly typed, but many critical paths still use stringly conventions (`format: &str`, ad-hoc coordinate/unit parsing, `Box<dyn Error>` everywhere), and one path actively degrades known typed data back into an `Unknown` variant. Together, this increases bug risk and makes long-term refactoring more expensive.

## Critical Findings
### Known polygon type serialized as `Unknown`
- **Severity**: critical
- **Category**: Using Known Types as Unknown
- **Location(s)**: `crates/altium-format/src/ops/pcblib.rs:3127-3130`, `crates/altium-format/src/ops/pcblib.rs:3378-3386`
- **Problem**: `PcbRecord::Polygon(_)` is explicitly mapped to `PcbPrimitiveJson::Unknown`, even though the type is known at conversion time.
- **Evidence**:
  - `PcbRecord::Polygon(_) => PcbPrimitiveJson::Unknown { ... }`
  - `Unknown` JSON decoding uses `base64` decode with `unwrap_or_default()`, silently swallowing decode failures and producing empty payloads.
- **Recommendation**: Add a real `Polygon` variant to `PcbPrimitiveJson` and map both directions explicitly (`PcbRecord::Polygon <-> PcbPrimitiveJson::Polygon`). Treat base64 decode failures as hard errors (`Result`) rather than silent defaults.

### Error type soup across CLI and ops layers
- **Severity**: critical
- **Category**: Rust Antipatterns (Error type soup)
- **Location(s)**: `crates/altium-cli/src/main.rs`, all `crates/altium-cli/src/commands/*.rs` `run` signatures, and many `crates/altium-format/src/ops/*.rs` command functions.
- **Problem**: Most public command paths return `Result<_, Box<dyn std::error::Error>>`, while the format crate already defines a structured `AltiumError` (`thiserror`). This loses typed failure semantics and forces string-based context propagation.
- **Evidence**:
  - `fn main() -> Result<(), Box<dyn std::error::Error>>`
  - `pub fn run(...)-> Result<(), Box<dyn std::error::Error>>` in command modules
  - `cmd_*` ops APIs frequently returning boxed trait objects
- **Recommendation**: Standardize on a layered error model:
  1. `altium-format`: typed enums (`AltiumError` + sub-enums per ops domain where needed)
  2. `altium-cli`: one `CliError` enum with `From` conversions from domain errors
  3. Preserve rich context using variants, not `format!(...)` strings.

## Warning Findings
### God modules with mixed concerns (I/O + domain + presentation)
- **Severity**: warning
- **Category**: Architecture & Design / Separation of Concerns
- **Location(s)**: `crates/altium-format/src/ops/pcbdoc.rs` (~2688 LOC), `crates/altium-format/src/ops/pcblib.rs` (~3475 LOC), `crates/altium-cli/src/commands/pcbdoc.rs` (~1191 LOC).
- **Problem**: Large files aggregate command parsing assumptions, domain transforms, filtering, validation, and output-shaping logic. This makes ownership and abstraction boundaries unclear.
- **Evidence**:
  - `ops/pcbdoc.rs` contains both parsing helpers (`parse_coord`) and high-level command orchestration.
  - `commands/pcbdoc.rs::run` is a very large dispatch function with many branches and repeated output plumbing.
- **Recommendation**: Split by capability slices (e.g., `rules`, `routing`, `regions`, `placement`, `settings`) with a strict boundary:
  - IO adapters (open/save)
  - domain services (pure transforms)
  - CLI adapters (arg/format translation)

### Clone-heavy dispatch and repetitive orchestration
- **Severity**: warning
- **Category**: Rust Antipatterns (Clone abuse), DRY Violations
- **Location(s)**: `crates/altium-cli/src/commands/pcbdoc.rs` `run()` match arms.
- **Problem**: Numerous `.clone()` calls in dispatch are used to satisfy ownership at callsites. Repeated command wiring patterns (compute result -> `output::print`) also create boilerplate drift risk.
- **Evidence**:
  - Repeated `kind.clone()`, `layer.clone()`, `gap.clone()`, `at.clone()`, etc. in many branches.
  - Nearly identical blocks for list/detail commands.
- **Recommendation**: Introduce small helper adapters and pass references where possible (`&Option<String>`, `&str`, domain enums). Use a macro or generic helper only for truly repeated output patterns.

### Stringly-typed formatting contract in core CLI path
- **Severity**: warning
- **Category**: Rust Antipatterns (Stringly-typed data)
- **Location(s)**: `crates/altium-cli/src/main.rs`, `crates/altium-cli/src/output.rs`.
- **Problem**: Output mode is represented as raw `&str` values (`"text"`, `"json"`, `"json-pretty"`) and validated with runtime string matching.
- **Evidence**:
  - `let format = if ... { "json-pretty" } else { "json" } ...`
  - `match format { "text" | "json" | "json-pretty" ... _ => Err("Unknown format") }`
- **Recommendation**: Replace with an `enum OutputFormat { Text, Json, JsonPretty }` and thread typed values through command APIs. This removes impossible states and makes exhaustive matching compile-time enforced.

### Risky panic and unchecked numeric conversion patterns in binary parsing/writing
- **Severity**: warning
- **Category**: Rust Antipatterns (`unwrap`/casts)
- **Location(s)**: `crates/altium-format/src/footprint/measure.rs:318`, `crates/altium-format/src/v2/pcb/pad.rs` (multiple `try_into().unwrap()`), `crates/altium-format/src/v2/serializer/binary.rs` (many `as` casts).
- **Problem**: Several non-test paths rely on `unwrap()` or potentially truncating casts. While some are guarded by length checks, the pattern invites edge-case panics and silent truncation.
- **Evidence**:
  - `partial_cmp(...).unwrap()` in pitch sorting (NaN can panic)
  - binary serialization paths cast lengths/integers with `as`
- **Recommendation**: Replace with checked conversions (`try_into`, `checked_*`) and explicit error returns on invalid values. For float ordering use `total_cmp` or filter NaN before sort.

## Note Findings
### Collect-then-iterate patterns add allocations and noise
- **Severity**: note
- **Category**: Rust Antipatterns (Collect-then-iterate)
- **Location(s)**: `crates/altium-format/src/ops/pcbdoc.rs` `cmd_rules` and similar list operations.
- **Problem**: Intermediate vectors are collected then immediately iterated/mapped again, increasing allocation and code verbosity.
- **Recommendation**: Streamline iterator chains directly into final target collections unless intermediate materialization is required for sorting/reuse.

### Incomplete feature TODOs in serializer paths
- **Severity**: note
- **Category**: General bad judgment & codebase awareness
- **Location(s)**: `crates/altium-format/src/v2/serializer/ascii.rs`, `crates/altium-format/src/v2/serializer/binary.rs`.
- **Problem**: TODO markers in serialization code indicate known behavior gaps (`hex+zlib`, `Real48 angles`) in core format infrastructure.
- **Recommendation**: Track these with issue links and compatibility tests so these known gaps are explicit in release quality criteria.

## Systemic Patterns
1. **Boundary dilution**: command handlers and ops modules combine too many responsibilities.
2. **Type inconsistency**: strong domain types coexist with stringly control values and boxed trait-object errors.
3. **Repetition under scale**: each new command path copies orchestration patterns rather than extending shared typed pipelines.
4. **Round-trip fidelity risk**: unknown/generic fallback handling is used in places where concrete types are available.

## Architecture Assessment
Dependency flow is broadly straightforward (`altium-cli -> altium-format -> derive/helpers`), and there are no obvious crate-level cycles. The issue is intra-crate layering: `ops` behaves as a monolithic application layer with weak internal seams. Domain logic, parse/format adaptation, and command semantics are interleaved in the same units.

The data model is promising in lower layers (`Coord`, `Layer`, record enums), but higher-level APIs undermine this by accepting or returning untyped strings and trait-object errors. That reduces the value of the type model exactly where correctness-sensitive user interactions happen.

A modularization pass that extracts cohesive domain services and introduces typed request/response models per command family would significantly improve maintainability and testability without changing external CLI behavior.

## Recommendations (Prioritized)
1. **Fix `Polygon -> Unknown` degradation path immediately** and add round-trip tests asserting polygon fidelity through JSON conversion.
2. **Unify error strategy** around typed enums (`AltiumError` + `CliError`) and remove `Box<dyn Error>` from public command APIs.
3. **Introduce `OutputFormat` enum** and remove stringly format dispatch across CLI and output modules.
4. **Decompose giant `ops`/`commands` files** into capability-focused modules with strict boundaries between IO adapters and pure domain logic.
5. **Reduce clone/repetition in dispatchers** with borrowed parameters and shared output helpers.
6. **Harden binary parsing** by eliminating non-test `unwrap()` and risky `as` casts in favor of checked conversions.
