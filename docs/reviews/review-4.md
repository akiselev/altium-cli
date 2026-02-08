# Codebase Review: altium-cli

## Executive Summary
This workspace has a strong foundation: the crate split (`altium-cli`, `altium-format`, `altium-format-derive`) is sensible, and crate-level dependencies are clean.

The biggest systemic issue is boundary erosion between domain, transport, and CLI layers. Several modules blend parsing, domain transformation, formatting, and output concerns into long command files. This has already produced correctness debt: one path converts a known `Polygon` primitive into an `Unknown` placeholder and silently drops payload.

Secondary systemic concerns are type precision drift (stringly-typed modes), inconsistent error handling (`Box<dyn Error>` in CLI), and clone-heavy flows in query paths.

## Critical Findings
### Polygon roundtrip degrades known type into unknown and loses data
- **Severity**: critical
- **Category**: Using Known Types as Unknown
- **Location(s)**: `crates/altium-format/src/ops/pcblib.rs:3127-3130`, `crates/altium-format/src/ops/pcblib.rs:2890-2904`, `crates/altium-format/src/ops/pcblib.rs:3378-3386`
- **Problem**: `PcbRecord::Polygon` is mapped to `PcbPrimitiveJson::Unknown` with empty `raw_data`. Known typed data is forced into an unknown channel and cannot roundtrip safely.
- **Evidence**:
  - `PcbRecord::Polygon(_) => PcbPrimitiveJson::Unknown { ... raw_data: String::new() }`
  - JSON enum has no dedicated polygon variant.
- **Recommendation**: Add a first-class `Polygon` JSON variant and bidirectional conversion logic.

### Silent decode fallback masks corrupted JSON payloads
- **Severity**: critical
- **Category**: Rust Antipatterns (unwrap/expect misuse)
- **Location(s)**: `crates/altium-format/src/ops/pcblib.rs:3383-3386`
- **Problem**: malformed base64 is silently converted into empty bytes via `unwrap_or_default()`, hiding corruption.
- **Evidence**: `decode(raw_data).unwrap_or_default()`.
- **Recommendation**: propagate decode failures as typed errors; include primitive index context.

## Warning Findings
### CLI command layer is oversized and mixes concerns
- **Severity**: warning
- **Category**: Separation of Concerns Violations / Architecture & Design
- **Location(s)**: `crates/altium-cli/src/main.rs:150-215`, `crates/altium-cli/src/commands/mod.rs:3-12`, `crates/altium-cli/src/commands/query.rs:117-195`
- **Problem**: command handlers perform I/O, domain work, aggregation, timing, and rendering. This tightly couples CLI transport with business logic.
- **Evidence**: `query.rs` opens files, constructs trees, executes queries, builds output DTOs, and prints in one flow.
- **Recommendation**: add an application service layer returning typed results; keep command modules thin adapters.

### Error handling is unstructured in CLI surface
- **Severity**: warning
- **Category**: Rust Antipatterns (Error type soup)
- **Location(s)**: `crates/altium-cli/src/main.rs:150`, `crates/altium-cli/src/commands/query.rs:98`, `crates/altium-cli/src/commands/query.rs:121`, `crates/altium-cli/src/commands/query.rs:162`
- **Problem**: pervasive `Result<_, Box<dyn std::error::Error>>` erases semantics and blocks differentiated handling.
- **Evidence**: top-level and command `run` functions use boxed dynamic errors.
- **Recommendation**: standardize on `CliError` (`thiserror`) and return `Result<_, CliError>`.

### Stringly-typed modes should be enums
- **Severity**: warning
- **Category**: Rust Antipatterns (Stringly-typed data)
- **Location(s)**: `crates/altium-cli/src/main.rs:153-157`, `crates/altium-cli/src/commands/query.rs:98`, `crates/altium-format/src/ops/pcblib.rs:2908-2913`, `crates/altium-format/src/ops/pcblib.rs:2931-2945`
- **Problem**: output format and mask mode are stored as raw strings, allowing invalid values and repeated string comparisons.
- **Evidence**:
  - `format` is passed as `&str` (`"text"`, `"json"`, `"json-pretty"`).
  - `PcbMaskExpansionJson` uses `mode: String` with runtime branch on string content.
- **Recommendation**: use enums (`OutputFormat`, `MaskExpansionMode`) with serde tagging.

### Clone-heavy query path likely allocates unnecessarily
- **Severity**: warning
- **Category**: Rust Antipatterns (Clone abuse)
- **Location(s)**: `crates/altium-cli/src/commands/query.rs:132`, `crates/altium-cli/src/commands/query.rs:171`
- **Problem**: query execution clones full primitive vectors to build trees (`doc.primitives.clone()`, `component.primitives.clone()`). For large designs/libraries this increases memory traffic.
- **Evidence**: full collection clone before every tree build.
- **Recommendation**: build `RecordTree` from borrowed data or use an index/arena shared by query operations.

### Cache accessors duplicate logic and use `contains_key` + `get(...).unwrap()`
- **Severity**: warning
- **Category**: DRY Violations + Rust Antipatterns
- **Location(s)**: `crates/altium-format/src/api/document.rs:65-105`
- **Problem**: four methods duplicate load/insert/get logic and do double hash lookups with post-check unwrap.
- **Evidence**: repeated `contains_key` branch followed by `get(...).unwrap()`/`get_mut(...).unwrap()`.
- **Recommendation**: refactor via `entry` API helper to eliminate duplication and unwraps.

### V2 component model is a god struct with weak invariants
- **Severity**: warning
- **Category**: Rust Antipatterns (God structs) / Data model mismatch
- **Location(s)**: `crates/altium-format/src/v2/fields/component.rs:10-81`
- **Problem**: `ComponentData` contains 47+ public fields spanning identity, graphical state, vault metadata, and rendering flags. It is hard to validate and easy to construct invalid combinations.
- **Evidence**: monolithic struct with broad, all-`pub` field surface.
- **Recommendation**: split into cohesive sub-structs (`Identity`, `Placement`, `Display`, `VaultRefs`), keep fields private, and expose validated constructors.

## Note Findings
### Very broad public API surface in core crate
- **Severity**: note
- **Category**: Leaky abstractions via `pub`
- **Location(s)**: `crates/altium-format/src/lib.rs:91-122`
- **Problem**: most internal modules are publicly exported, increasing compatibility burden and limiting refactoring freedom.
- **Evidence**: `pub mod` for many subsystems plus broad type re-exports.
- **Recommendation**: narrow surface to stable entry points; move internals behind `pub(crate)` where possible.

## Systemic Patterns
1. **Transport/domain coupling**: CLI handlers directly orchestrate file I/O + domain operations + rendering.
2. **Type precision leakage**: string flags and generic unknown channels used where explicit domain variants should exist.
3. **Error semantics erosion**: boxed dynamic errors at boundaries and silent fallback patterns reduce diagnosability.
4. **Scalability headwinds**: clone-heavy paths and very large modules make performance and maintenance progressively harder.

## Architecture Assessment
At crate level, architecture is good: a CLI facade over a reusable format library plus derive crate is a practical decomposition. Inside crates, boundaries are less clean. `altium-format` contains both low-level parsing and high-level ops/templates/query APIs, while `altium-cli` command modules frequently perform orchestration and domain behavior inline.

The dependency flow is mostly acyclic, but module responsibilities are too broad in several areas (`ops/*`, `commands/*`). Stronger layering (parsers -> domain model -> use-cases -> CLI/render) would reduce coupling and improve testability.

## Recommendations (Prioritized)
1. **Fix polygon/unknown roundtrip immediately**: add typed polygon JSON support and fail-fast decode errors.
2. **Introduce structured CLI errors**: replace `Box<dyn Error>` with `CliError` enums and preserve context.
3. **Create an application service layer**: move business workflows out of command handlers.
4. **Replace string mode flags with enums**: start with output format + mask expansion mode.
5. **Refactor duplicate cache loaders in `api::document`**: use `entry` helpers and remove unwraps.
6. **Decompose `ComponentData` and reduce pub exposure**: improve invariants and long-term API maintainability.
