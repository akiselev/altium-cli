# Codebase Review: altium-cli

## Executive Summary
The workspace has a clear high-level split between CLI entrypoints (`altium-cli`) and format/domain operations (`altium-format`), and the project already includes strong building blocks like a typed `AltiumError` plus extensive record modeling. The breadth of supported Altium artifacts is impressive, and much of the record parsing is explicit and readable.

The biggest systemic problem is boundary erosion: command handlers, operational modules, and domain logic are interleaved. This shows up as repeated string parsing, duplicated command semantics across crates, broad `Box<dyn Error>` signatures, and ad-hoc conversion layers. The result is high change cost and subtle correctness risk when behavior is modified in one place but not another.

The most urgent correctness issue is in parsing and query semantics: component parsing in `PcbDoc` silently truncates on first malformed component, and selector wildcard operators are translated through a lossy operator mapping. Those two issues can produce incorrect output without hard failures, which is worse than explicit errors for an engineering tool.

## Critical Findings
### Silent component truncation in `PcbDoc` reader
- **Severity**: critical
- **Category**: Architecture & Design / Correctness
- **Location(s)**: `crates/altium-format/src/io/pcbdoc.rs` (around lines 157-163)
- **Problem**: Component iteration swallows any component parse error and breaks the loop. A single malformed component causes the rest of the component stream to be ignored, returning a partial document without surfacing an error.
- **Evidence**:
  - `while ... { match self.read_component_record(...) { Ok(comp) => push, Err(_) => break } }`
- **Recommendation**: Treat parse failure as structured error with context (offset/index), or collect recoverable errors and continue with explicit diagnostics. Do not silently stop parsing.

### Lossy wildcard operator mapping in selector engine
- **Severity**: critical
- **Category**: Rust antipatterns / Architecture & Design
- **Location(s)**:
  - `crates/altium-format/src/query/selector.rs` (around lines 348-354)
  - `crates/altium-format/src/query/common.rs` (around lines 166-179)
- **Problem**: `FilterOperator::Wildcard` is converted to `FilterOp::WordMatch` (“closest equivalent”), but `WordMatch` only performs whitespace token equality. Wildcard/pattern behavior is semantically different and can return wrong query results.
- **Evidence**:
  - `selector.rs`: `Self::Wildcard => FilterOp::WordMatch // Closest equivalent`
  - `common.rs`: `WordMatch` implementation splits on whitespace and compares plain strings.
- **Recommendation**: Unify the filter operator/value model and add explicit `PatternMatch` semantics in `FilterOp` instead of coercing through `WordMatch`.

## Warning Findings
### Stringly-typed command model in edit pipeline
- **Severity**: warning
- **Category**: Rust antipatterns (Stringly-typed data)
- **Location(s)**: `crates/altium-cli/src/commands/edit.rs` (operation enum and parser/match blocks)
- **Problem**: `EditOperation` stores `style`, `orientation`, and `io_type` as `String`, then later remaps those strings into enums in executor functions. This allows invalid state to exist after parse and scatters validation.
- **Evidence**:
  - `EditOperation::AddPower { style: String, orientation: String }`
  - later `match style.to_lowercase().as_str()` and `match io_type.to_lowercase().as_str()`
- **Recommendation**: Parse into typed enums at command parsing time (`FromStr` implementations), and keep `EditOperation` fully typed.

### DRY violation: duplicated edit semantics across CLI and ops layer
- **Severity**: warning
- **Category**: DRY violations
- **Location(s)**:
  - `crates/altium-cli/src/commands/edit.rs`
  - `crates/altium-format/src/ops/schdoc_edit.rs`
- **Problem**: Power style/orientation/port-I/O parsing and error messages are duplicated almost verbatim in two modules. This guarantees drift.
- **Evidence**:
  - Same power style match arms and same “Unknown power style...” messages in both files.
  - Same orientation and I/O type parsing blocks in both files.
- **Recommendation**: Move parsing/validation into shared typed helpers (single source of truth), expose reusable functions or types from `altium-format`.

### Error type soup across command and ops surfaces
- **Severity**: warning
- **Category**: Rust antipatterns (Error type soup)
- **Location(s)**:
  - `crates/altium-format/src/error.rs`
  - many `run`/`cmd_*` functions in `crates/altium-cli/src` and `crates/altium-format/src/ops/*`
- **Problem**: The codebase defines a structured `AltiumError`, but large parts of CLI and ops return `Box<dyn Error>`. This erases error taxonomy and weakens diagnostics.
- **Evidence**:
  - `AltiumError` enum exists and is rich.
  - Command signatures broadly use `Result<_, Box<dyn std::error::Error>>`.
- **Recommendation**: Standardize on typed errors at crate boundaries (`thiserror` enums), use explicit conversion only at final CLI boundary.

### Separation-of-concerns breach is acknowledged but still entrenched
- **Severity**: warning
- **Category**: Separation of Concerns violations
- **Location(s)**: `crates/altium-format/src/ops/pcblib.rs`
- **Problem**: ops functions combine I/O, domain transforms, and output concerns. A top-file comment acknowledges this debt, but the module keeps expanding under that pattern.
- **Evidence**:
  - Inline comment: cmd functions mix presentation and business logic.
  - Module imports parsing/rendering/output concerns simultaneously.
- **Recommendation**: Split into `service` (pure operations), `adapter` (file I/O), and `presentation` (CLI formatting).

### Leaky public data model in v2 field structs
- **Severity**: warning
- **Category**: Rust antipatterns (Leaky abstractions via `pub`)
- **Location(s)**: `crates/altium-format/src/v2/fields/primitives.rs` (and similar v2 field modules)
- **Problem**: Record structs expose all fields as mutable `pub`, including low-level serialization details. This makes invariants unenforceable and freezes internal representation as public API.
- **Evidence**:
  - `ArcData`, `RectangleData`, `ImageData` etc. have all-`pub` fields.
- **Recommendation**: Restrict visibility (`pub(crate)` or private), add constructors/builders validating invariants, expose read-only accessors where needed.

## Note Findings
### Incomplete serializer TODOs in production path
- **Severity**: note
- **Category**: Dead code / technical debt awareness
- **Location(s)**:
  - `crates/altium-format/src/v2/serializer/binary.rs`
  - `crates/altium-format/src/v2/serializer/ascii.rs`
- **Problem**: Serialization paths contain unresolved TODOs for encoding behavior, risking hidden format incompatibilities.
- **Evidence**:
  - TODO comments for Real48 angles and hex+zlib encoding.
- **Recommendation**: Track with issues + tests + feature flags so incomplete behavior is explicit to users.

## Systemic Patterns
1. **String-first interfaces at boundaries**: CLI and ops parse free-form strings deeply into execution paths rather than normalizing to typed command DTOs once.
2. **Parallel abstractions**: Query system has overlapping operator/value layers that require conversion glue and semantic compromises.
3. **Boundary erosion**: I/O, business logic, and output formatting are repeatedly mixed in the same modules.
4. **Inconsistent error strategy**: Typed errors exist but are bypassed in many public functions, leading to weak observability.

## Architecture Assessment
The dependency direction is mostly healthy at crate granularity (`altium-cli` depends on `altium-format`), but inside `altium-format` the `ops` layer is acting as a catch-all application layer rather than a thin façade. Query functionality is powerful but currently split into two models with incomplete unification, which increases maintenance load and semantic drift risk.

The type inventory is rich, but several surfaces still behave like untyped scripting interfaces (string commands and broad dynamic errors). This mismatch creates a codebase that *looks* strongly typed internally while exposing weakly typed pathways at high-traffic boundaries.

## Recommendations (Prioritized)
1. **Fix correctness first**: remove silent truncation in `PcbDoc::read_components` and add tests for malformed mid-stream component records.
2. **Unify query filter semantics**: introduce one operator/value model with explicit wildcard/pattern behavior and delete lossy conversions.
3. **Normalize typed command parsing**: convert edit/query argument parsing to typed enums/newtypes at parse boundary.
4. **Consolidate shared edit semantics**: factor duplicated power/orientation/I/O parsing into reusable library functions.
5. **Adopt typed error policy**: `AltiumError`/`CliError` internally, only map to printable text at the outermost command boundary.
6. **Reduce API leakage**: tighten `pub` fields in `v2` models, introduce invariants via constructors and tests.
