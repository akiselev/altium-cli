# Codebase Review: altium-cli

## Executive Summary
The workspace-level architecture is directionally good: `altium-format-derive` (proc macros) feeds `altium-format` (domain + parsing) which feeds `altium-cli` (interface), and `cargo tree` confirms that layering. That separation is the strongest part of this codebase, and it gives you a maintainable release pipeline foundation.

The core maintainability issue is not crate-level layering; it is **intra-crate boundary erosion**. The CLI command modules have become very large orchestration-and-business-logic hybrids, while operation functions in `altium-format::ops` return `Box<dyn Error>` and perform mixed concerns (I/O + transformation + formatting-oriented payload assembly). Over time this will increase change amplification and make behavior-level testing expensive.

The most important correctness issue found is in unknown-field preservation: `UnknownFields::remove_param` reports success without actually removing the parameter from the stored collection. That silently violates API expectations and can lead to stale data being re-emitted during round-trip serialization.

## Critical Findings

### `UnknownFields::remove_param` has incorrect semantics (returns success without deleting data)
- **Severity**: critical
- **Category**: Rust antipatterns / correctness bug
- **Location(s)**: `crates/altium-format/src/types/unknown.rs:157-166`
- **Problem**: The method name and boolean return imply data removal, but only `param_order` is updated; the underlying `ParameterCollection` entry remains.
- **Evidence**:
  - `self.param_order.retain(|k| k != key);`
  - explicit comment: `ParameterCollection doesn't have remove ... For now, we just remove from order tracking`.
  - method still returns `true`.
- **Recommendation**: Make behavior truthful and atomic. Either:
  1. add `remove` support to `ParameterCollection` and truly remove from both containers, or
  2. rename method to `hide_param_from_order` and document non-removal semantics (not recommended for public API), or
  3. rebuild `ParameterCollection` without the key before returning.

## Warning Findings

### Stringly-typed command output and file type dispatch
- **Severity**: warning
- **Category**: Rust antipatterns (Stringly-typed data)
- **Location(s)**:
  - `crates/altium-cli/src/main.rs:153-157,198-204`
  - `crates/altium-cli/src/output.rs:18-37`
  - `crates/altium-cli/src/commands/query.rs:99-113`
  - `crates/altium-cli/src/commands/inspect.rs:49-57`
- **Problem**: Format and file type routing rely on ad-hoc string literals (`"text"`, `"json-pretty"`, extension strings, shell names). This prevents compiler-checked exhaustiveness and spreads magic strings across modules.
- **Evidence**: `match format { "text" | "json" | "json-pretty" ... }`, extension matching via `to_lowercase`, shell matching by `String`.
- **Recommendation**: Introduce typed enums (`OutputFormat`, `InputFileType`, `ShellKind`) with `clap::ValueEnum` / `TryFrom<&str>`, then centralize parsing at boundaries.

### Error type soup across CLI and ops layers (`Box<dyn Error>` as de facto default)
- **Severity**: warning
- **Category**: Rust antipatterns (Error type soup)
- **Location(s)**:
  - `crates/altium-cli/src/main.rs:150`
  - `crates/altium-cli/src/output.rs:21`
  - Broadly across command and ops entrypoints (e.g., `crates/altium-cli/src/commands/*.rs`, `crates/altium-format/src/ops/*.rs`)
- **Problem**: Public-facing command and ops APIs largely erase structured error information into `Box<dyn Error>`, while the library already has a typed `AltiumError`. This hurts diagnostics, makes mapping to CLI exit codes brittle, and obscures recoverable-vs-fatal distinctions.
- **Evidence**: widespread signatures like `Result<_, Box<dyn std::error::Error>>` in command/ops functions.
- **Recommendation**: Standardize on typed error enums (`thiserror`) per crate/layer, with explicit conversion boundaries. Keep trait-object errors only at the true process boundary (`main`) if necessary.

### Clone-heavy query path creates avoidable allocations
- **Severity**: warning
- **Category**: Rust antipatterns (Clone abuse)
- **Location(s)**: `crates/altium-cli/src/commands/query.rs:132,171`
- **Problem**: Query operations clone full primitive collections just to build temporary trees. On large docs/libs this amplifies memory and CPU cost for read-only workflows.
- **Evidence**: `RecordTree::from_records(doc.primitives.clone())` and `RecordTree::from_records(component.primitives.clone())` inside per-query/per-component execution.
- **Recommendation**: Add borrowed tree construction (`from_records_ref`) or refactor query engine to consume iterators/borrowed slices.

### God modules in CLI command layer indicate missing abstraction boundaries
- **Severity**: warning
- **Category**: Architecture & design / Separation of concerns
- **Location(s)**:
  - `crates/altium-cli/src/commands/pcbdoc.rs` (~1191 LOC)
  - `crates/altium-cli/src/commands/edit.rs` (~829 LOC)
  - `crates/altium-cli/src/commands/pcblib.rs` (~799 LOC)
  - `crates/altium-cli/src/commands/schdoc.rs` (~734 LOC)
- **Problem**: Large command modules blend argument structs, dispatch routing, output shaping, and business-level operation wiring. This increases cognitive load and causes high churn in single files.
- **Evidence**: mega-match dispatch in `pcbdoc::run` with repetitive `output::print` and command-specific mapping (`crates/altium-cli/src/commands/pcbdoc.rs:733+`).
- **Recommendation**: Split by subdomain (read-only queries vs mutating ops vs formatting), and introduce common dispatch helpers/macros for repeated result-printing pattern.

### Query language detection is heuristic and fragile
- **Severity**: warning
- **Category**: KISS / wrong abstraction boundary
- **Location(s)**: `crates/altium-cli/src/commands/query.rs:198-225`
- **Problem**: `is_schql_query` infers language via substring heuristics (`contains(':')`, `contains('>')`, etc.). This can misclassify edge inputs and decouples language selection from parser truth.
- **Evidence**: boolean heuristic over lowercased input before parser execution.
- **Recommendation**: Attempt parse with one grammar and fall back to the other based on structured parse errors (or require explicit `--lang` with optional auto mode).

## Note Findings

### Serializer TODOs represent partially implemented behavior in core format logic
- **Severity**: note
- **Category**: Dead code / maintainability debt
- **Location(s)**:
  - `crates/altium-format/src/v2/serializer/ascii.rs:545-548`
  - `crates/altium-format/src/v2/serializer/binary.rs:183-185`
- **Problem**: Core serializer paths carry TODO stubs (ASCII binary-data export, Real48 angle handling), which can silently degrade fidelity.
- **Recommendation**: Track as explicit issues with test coverage gates and feature flags (or hard errors) to avoid shipping "pretend success" behavior.

### Inconsistent use of typed vs stringly domain identifiers
- **Severity**: note
- **Category**: Data model consistency
- **Location(s)**: `crates/altium-cli/src/commands/query.rs`, `crates/altium-cli/src/commands/inspect.rs`, `crates/altium-cli/src/main.rs`
- **Problem**: Record/file/language identifiers are represented by raw strings in many user-facing structs.
- **Recommendation**: Promote common enums/newtypes for identifiers used across command output and dispatch logic.

## Systemic Patterns
1. **Boundary slippage from typed core to stringly shell**: internally there are strong domain types, but CLI/ops layers repeatedly collapse to `String` and `Box<dyn Error>`.
2. **Command-module monolith growth**: command files accumulate dispatch + transformation + formatting responsibilities, causing repetition and high change coupling.
3. **Performance by cloning**: immutable workflows frequently materialize cloned record vectors rather than borrowing.
4. **Best-effort fallbacks in critical paths**: TODO stubs and heuristic detection prioritize permissive behavior over explicit failures, risking silent correctness drift.

## Architecture Assessment
- **What is working**:
  - Workspace crate layering is clean and intentional (derive → format → cli).
  - Presence of a dedicated `types` module and `thiserror`-based `AltiumError` in core is a good base.
- **What is degrading**:
  - The logical domain layer in `altium-format` is not cleanly separated from I/O-oriented ops payload construction.
  - CLI commands are coupled directly to low-level operation details and formatting choices.
  - Repetition in command dispatch indicates missing reusable command execution abstractions.
- **Overall**: good macro-architecture, weak micro-architecture in the application/service layers.

## Recommendations (Prioritized)
1. **Fix `UnknownFields::remove_param` correctness immediately** and add regression tests for true deletion semantics.
2. **Standardize error handling**: introduce typed error enums in `ops` and CLI command layers; confine `Box<dyn Error>` to `main` boundary only.
3. **Replace stringly dispatch with enums** (`OutputFormat`, file type, query language, shell type) and central parse/validation.
4. **Refactor command monoliths**: split `pcbdoc.rs`/`edit.rs` by use case and extract shared dispatch-print helpers.
5. **Eliminate hot-path clones in queries** through borrowed tree/query APIs.
6. **Convert parser TODOs/heuristics into explicit behavior contracts** (implemented paths or hard errors with tests).
