# Codebase Review: altium-cli

## Executive Summary
The workspace has a solid high-level split between CLI (`altium-cli`), format/parsing (`altium-format`), and proc-macro support (`altium-format-derive`), and the project already uses good ecosystem choices (`clap`, `serde`, `thiserror`). The codebase is also unusually explicit about binary format behavior, which is valuable for reverse-engineered file formats.

The biggest systemic risk is that several core models are trying to be both **lossless raw containers** and **typed domain objects** at once. That design creates invalid states and forces broad, mutable structs with weak invariants. The second large issue is a recurring stringly-typed boundary in the CLI and command layer, which reduces compiler assistance and spreads parsing/validation logic across many files.

A third issue is maintainability debt from very large, mixed-responsibility modules (`ops/schlib.rs`, CLI command handlers) and duplicated mapping logic. This does not fail correctness immediately, but it will make feature additions progressively slower and riskier.

## Critical Findings
### 1) Component Body Is Parsed as Region (known type represented as generic type)
- **Severity**: critical
- **Category**: Using Known Types as Unknown
- **Location(s)**: `crates/altium-format/src/v2/pcb/io/pcblib.rs` (struct fields and parse dispatch)
- **Problem**: `ComponentBody` objects are stored in `component_bodies: Vec<PcbRegion>` and parsed with `PcbRegion::read_from` even when the object id is explicitly `PcbObjectId::ComponentBody`. This bakes a known domain type into a more generic surrogate type.
- **Evidence**:
  - `pub component_bodies: Vec<PcbRegion>` in `PcbLibFootprint`.
  - `PcbObjectId::ComponentBody => match PcbRegion::read_from(&block)`.
- **Recommendation**: Introduce a dedicated `PcbComponentBody` type (or at least a newtype wrapper around the shared payload) and parse/serialize via that type under the `ComponentBody` branch. Keep conversions explicit if binary layouts are currently identical.

### 2) `PcbLibFootprint` mixes raw roundtrip payload and typed model, creating invalid states
- **Severity**: critical
- **Category**: Ignoring the type system / Data model doesn’t match domain / Separation of concerns
- **Location(s)**: `crates/altium-format/src/v2/pcb/io/pcblib.rs` (model + write path)
- **Problem**: `PcbLibFootprint` contains `primitive_count`, typed primitive vectors, raw primitive bytes, raw ordering, raw parameter bytes, and parsed parameters simultaneously. These can contradict each other; `write()` resolves conflicts with fallback logic rather than type invariants.
- **Evidence**:
  - One struct has both typed and raw representations (`tracks`, `pads`, `raw_primitives`, `primitive_order`, `parameters`, `raw_parameters`).
  - `write()` recomputes count from either `primitive_order` or summed vectors instead of relying on a single canonical model.
- **Recommendation**: Split representation into explicit states, e.g. `RawFootprint` vs `TypedFootprint`, or enforce invariants with a builder/conversion step (`ParsedFootprint -> EditableFootprint`). Avoid storing two authoritative sources in the same mutable struct.

## Warning Findings
### 3) Stringly-typed command and output modes are spread through the CLI
- **Severity**: warning
- **Category**: Stringly-typed data
- **Location(s)**: `crates/altium-cli/src/main.rs`, `crates/altium-cli/src/output.rs`, `crates/altium-cli/src/commands/edit.rs`
- **Problem**: output modes (`"text"`, `"json"`, `"json-pretty"`), shell names, and edit options (`style`, `orientation`, `io_type`) are passed around as `&str`/`String`. This leaks parsing concerns and allows invalid values to travel deep before failing.
- **Evidence**:
  - `format` computed as string literals in `main.rs` and matched again in `output::print`.
  - `Completions { shell: String }` then string matching.
  - `EditOperation::AddPower { style: String, orientation: String }` and `AddPort { io_type: String }`.
- **Recommendation**: Replace with enums (`OutputFormat`, `ShellKind`, `PowerStyleArg`, `OrientationArg`, `PortIoArg`) and parse once at the CLI boundary using `clap` value enums.

### 4) Command modules combine parsing, domain orchestration, I/O, rendering, and process control
- **Severity**: warning
- **Category**: Separation of concerns / Missing abstraction boundaries
- **Location(s)**: `crates/altium-cli/src/commands/edit.rs`, `crates/altium-cli/src/commands/query.rs`
- **Problem**: command modules perform argument parsing, open/save files, run domain logic, format outputs, and in one case call `std::process::exit(1)`. This makes command code hard to test and reuse and ties domain behavior to CLI runtime behavior.
- **Evidence**:
  - `edit::run` parses operation, executes edits, prints output, then exits process on failure.
  - `query` combines file opening, query execution, output-model transformation, and output printing in one module.
- **Recommendation**: Extract pure application services (`EditService`, `QueryService`) returning typed results. Keep CLI modules as thin adapters (parse input -> call service -> render output).

### 5) DRY violation: duplicated large `record_type_name` mappings
- **Severity**: warning
- **Category**: DRY violations / Inconsistent conventions
- **Location(s)**: `crates/altium-format/src/ops/util.rs`, `crates/altium-format/src/ops/schlib.rs`
- **Problem**: nearly identical `SchRecord -> &str` mapping exists in multiple places. New record variants can silently diverge if one table is updated and another is not.
- **Evidence**:
  - `ops/util.rs::record_type_name(...)`
  - `ops/schlib.rs::record_type_name(...)` repeats same variant list.
- **Recommendation**: Centralize mapping in one module (or trait impl) and reuse everywhere.

### 6) Giant modules are carrying too many responsibilities
- **Severity**: warning
- **Category**: Architecture & Design / KISS violations
- **Location(s)**: `crates/altium-format/src/ops/schlib.rs`, `crates/altium-cli/src/commands/pcbdoc.rs`, `crates/altium-cli/src/commands/edit.rs`
- **Problem**: very large files with mixed command handlers, formatting, data transforms, and tests produce high cognitive load and make safe refactors difficult.
- **Evidence**:
  - File sizes are very large (`schlib.rs` ~3500 lines, `pcbdoc.rs` ~1200 lines, `edit.rs` ~800 lines).
  - `schlib.rs` even documents that cmd functions currently mix presentation and business logic.
- **Recommendation**: Split by operation family (read/report/edit), move common transforms into dedicated modules, and keep command adapters thin.

## Note Findings
### 7) Numeric casts in binary coordinate conversion are lossy and unchecked
- **Severity**: note
- **Category**: Needless `as` casts
- **Location(s)**: `crates/altium-format/src/records/pcb/outline.rs`
- **Problem**: `f64 -> i32` and `i32 -> f64` via `as` rely on implicit truncation/rounding behavior and no range checks.
- **Evidence**:
  - `let x = reader.read_f64::<LittleEndian>()? as i32;`
  - `writer.write_f64::<LittleEndian>(point.x.to_raw() as f64)?;`
- **Recommendation**: Add explicit checked conversion helpers (including out-of-range error paths and intentional rounding policy).

## Systemic Patterns
- **Boundary types are too weak**: strings and ad-hoc raw payloads are used where enums/newtypes/stateful wrappers should encode invariants.
- **Roundtrip and domain concerns are entangled**: lossless preservation logic frequently shares the same structs used for editing/querying.
- **Large command-oriented files accumulate duplicated helper logic**: multiple locations reimplement record mapping and command workflows.
- **Error strategy is inconsistent at crate boundaries**: library uses structured `AltiumError`, but CLI command layer widely returns `Box<dyn Error>`, reducing structured handling and discoverability.

## Architecture Assessment
Dependency direction is mostly sane at crate level: CLI depends on format library, and derive macros are isolated. The problem is not crate coupling; it is **intra-crate layering**. In `altium-format`, `ops` and `v2/io` modules mix transport concerns (raw streams), parsing, and application-level behavior in the same data structures. In `altium-cli`, command handlers are effectively mini-applications rather than adapters.

The result is architecture that works but is hard to evolve safely: adding one primitive type or command behavior tends to require edits in several large files and repeated mapping tables. A cleaner domain layer with stronger types at boundaries would reduce this change surface substantially.

## Recommendations (Prioritized)
1. **Split raw-vs-typed PCB models** in `v2/pcb/io` (especially `PcbLibFootprint`) and enforce invariants through conversion APIs.
2. **Introduce typed boundary enums/newtypes** for output format, shell kind, and edit-mode arguments; parse once at CLI boundary.
3. **Fix known-type handling for component bodies** by introducing a dedicated `PcbComponentBody` type path.
4. **Decompose large command/ops files** into thin command adapters + reusable domain services.
5. **Centralize record type mapping** (`SchRecord -> name`) and remove duplicated tables.
6. **Standardize error handling in CLI** (single error enum or consistent wrapper) instead of broad `Box<dyn Error>` everywhere.
7. **Audit unchecked numeric casts** in binary I/O and replace with explicit conversion policy.
    