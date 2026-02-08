# Codebase Review: altium-cli

**Date:** 2026-02-08
**Scope:** Full workspace — `altium-format-derive`, `altium-format`, `altium-cli`
**Size:** 176 .rs files, ~82K lines of code

## Executive Summary

This is a well-structured Rust workspace implementing read/write support for Altium Designer's proprietary binary file formats. The three-crate architecture (derive macros → core library → CLI) is clean and appropriate. The derive macro system for record serialization is well-designed and eliminates large amounts of boilerplate. The query engine with CSS-like selectors is a genuinely good abstraction. The template system for programmatic component creation is thoughtfully designed with LLM-friendly JSON input.

The biggest systemic issues are:

1. **Misguided backwards compatibility** in a pre-release codebase with no users — deprecated functions, dual error types, and dead code preserved "for future use" add complexity for zero benefit.
2. **DRY violations** — `record_type_name` is copy-pasted 3x (with subtle inconsistencies), `parse_coord` is implemented 4 separate times, and there are several `open_*` helper functions that are identical modulo the type name.
3. **Mixed error handling strategy** — Some ops functions return `Result<T, String>`, others return `Result<T, Box<dyn std::error::Error>>`, and the library has a perfectly good `AltiumError` enum that the ops layer largely ignores.
4. **The v1/v2 parallel type hierarchy** — Two complete type systems for PCB records with no conversion bridge, no clear migration path, and no documentation about which one to use for what.
5. **Monolithic ops modules** — Files reaching 3,500 lines that mix file I/O, business logic, and presentation formatting, making them untestable in isolation.

What's done well: zero unwrap/expect in non-test code, minimal unsafe (confined to enum transmutes with bounds checks), good use of the type system for domain modeling (Coord, Layer, CoordPoint), clean CLI structure with clap derive, and solid non-destructive editing via UnknownFields preservation.

## Critical Findings

### 1. Backwards Compatibility Theatre in a Pre-Release Codebase

- **Severity**: critical
- **Category**: General Bad Judgment / KISS Violation
- **Location(s)**:
  - `crates/altium-format/src/footprint/builder.rs:797-827`
  - `crates/altium-format/src/edit/library.rs:553-575`
  - `crates/altium-format/src/ops/schdoc.rs:32-42`
  - `crates/altium-format-derive/src/record.rs:643-747`
- **Problem**: The codebase is at version 0.1.8 with no external users, yet it carries deprecated functions, `#[allow(dead_code)]` annotations with "reserved for future" comments, and dual implementations preserved for compatibility. This is pure complexity tax with no benefit.
- **Evidence**:

  `FootprintBuilder` has two build methods that do the exact same thing:
  ```rust
  // builder.rs:800-813 — deprecated, identical to build_deterministic
  #[deprecated(since = "0.1.0", note = "Use build_deterministic()...")]
  pub fn build(self) -> PcbComponent {
      PcbComponent { /* ... uuid::Uuid::new_v4() ... */ }
  }

  // builder.rs:818-827 — "deterministic" but takes unused _det: &mut ()
  pub fn build_deterministic(self, _det: &mut ()) -> PcbComponent {
      PcbComponent { /* ... uuid::Uuid::new_v4() ... same code ... */ }
  }
  ```

  `uuid_simple_deterministic` takes `_det: &mut ()` and calls `Uuid::new_v4()` — not deterministic at all:
  ```rust
  // library.rs:573 — "_det" is unused, function is not deterministic
  fn uuid_simple_deterministic(_det: &mut ()) -> String {
      uuid::Uuid::new_v4().simple().to_string()
  }
  ```

  `open_schdoc` exists in two variants in the same file for "old-style" vs "refactored" error types:
  ```rust
  // schdoc.rs:32-36
  /// Open schematic document with String error type (for old-style functions).
  fn open_schdoc(path: &Path) -> Result<SchDoc, String> { ... }

  // schdoc.rs:38-42
  /// Open schematic document with Box<dyn Error> error type (for refactored functions).
  fn open_schdoc_boxed(path: &Path) -> Result<SchDoc, Box<dyn std::error::Error>> { ... }
  ```

  Dead code preserved in proc macro crate "for potential future use":
  ```rust
  // record.rs:643-644
  // NOTE: These functions are preserved for potential future use when migrating PCB records
  #[allow(dead_code)]
  fn generate_sch_primitive(...) { /* 60 lines */ }
  #[allow(dead_code)]
  fn generate_pcb_primitive(...) { /* 35 lines */ }
  ```

- **Recommendation**: Delete all deprecated functions. Delete all `#[allow(dead_code)]` blocks. Pick one error type and use it everywhere. The `_det: &mut ()` parameter pattern is a premature abstraction for a "DeterminismContext" that doesn't exist — remove it. You have git history if you need anything back.

---

### 2. `record_type_name` Triplicated With Subtle Bugs

- **Severity**: critical
- **Category**: DRY Violation
- **Location(s)**:
  - `crates/altium-format/src/records/sch/primitive.rs:614-651` (method on `SchRecord`)
  - `crates/altium-format/src/ops/util.rs:42-79` (free function)
  - `crates/altium-format/src/ops/schlib.rs:52-89` (private function)
- **Problem**: Three copies of the same 33-arm match statement, with at least one inconsistency: `primitive.rs:631` returns `"NoErc"` while `util.rs:59` and `schlib.rs:69` return `"NoERC"`. This is a latent bug — any code path that depends on the string value (e.g., querying by record type name) will behave differently depending on which copy is called.
- **Evidence**:
  ```rust
  // primitive.rs:631
  SchRecord::NoErc(_) => "NoErc",

  // util.rs:59 and schlib.rs:69
  SchRecord::NoErc(_) => "NoERC",
  ```
- **Recommendation**: Delete the two free-function copies. Use `SchRecord::record_type_name()` everywhere. Fix the casing inconsistency. The method already exists on the type — the free functions add nothing.

---

### 3. `parse_coord` Implemented Four Times

- **Severity**: warning
- **Category**: DRY Violation
- **Location(s)**:
  - `crates/altium-format/src/ops/pcbdoc.rs:31-55`
  - `crates/altium-format/src/edit/pcb_placement.rs:388-419`
  - `crates/altium-format/src/templates/mod.rs:236-239`
  - `crates/altium-cli/src/commands/edit.rs:274-290`
- **Problem**: Four separate implementations of "parse a string with a unit suffix into a coordinate." They have slightly different feature sets (pcb_placement supports "in", pcbdoc doesn't; templates use a different approach via `parse_number_with_unit`; edit.rs uses `Unit::parse_with_unit`). This is exactly the kind of thing that diverges silently.
- **Evidence**:
  ```rust
  // pcbdoc.rs — supports "mil" and "mm", default is mils
  fn parse_coord(s: &str) -> Result<Coord, String> { ... }

  // pcb_placement.rs — supports "mm", "mil", and "in"
  pub fn parse_coord(s: &str) -> Result<Coord, String> { ... }

  // edit.rs — delegates to Unit::parse_with_unit (different approach entirely)
  fn parse_coordinate(s: &str) -> Result<f64, Box<dyn Error>> { ... }
  ```

  Note that `pcbdoc.rs` even imports the placement version with a rename:
  ```rust
  use crate::edit::pcb_placement::{parse_coord as placement_parse_coord, ...};
  ```
  …then defines its own `parse_coord` that does almost the same thing but without "in" support.

- **Recommendation**: Consolidate into a single `Coord::parse(s: &str) -> Result<Coord, AltiumError>` method on the `Coord` type, or a single `parse_coord` in the types module. All four call sites should use it. `Unit::parse_with_unit` already exists and could be the canonical implementation.

---

### 4. Inconsistent Error Types Across ops/ Module

- **Severity**: warning
- **Category**: Error Type Soup
- **Location(s)**:
  - `crates/altium-format/src/ops/schdoc.rs` — mixed `String` and `Box<dyn Error>` in same file
  - `crates/altium-format/src/ops/pcbdoc.rs` — uses `String`
  - `crates/altium-format/src/ops/schlib.rs` — uses `Box<dyn Error>`
  - `crates/altium-format/src/ops/pcblib.rs` — uses `Box<dyn Error>`
  - `crates/altium-format/src/error.rs` — defines `AltiumError` (largely unused by ops)
- **Problem**: There are three error strategies in play, sometimes within the same file. `schdoc.rs` has both `open_schdoc() -> Result<_, String>` and `open_schdoc_boxed() -> Result<_, Box<dyn Error>>` because functions are being migrated one at a time. The library defines `AltiumError` with `thiserror`, but the ops layer ignores it. The result is that callers at the CLI boundary must handle three different error shapes.
- **Evidence**:
  ```rust
  // schdoc.rs:52
  pub fn cmd_create(path: &Path, template: Option<PathBuf>) -> Result<(), String>
  // schdoc.rs:85
  pub fn cmd_overview(path: &Path) -> Result<SchDocOverview, Box<dyn std::error::Error>>
  // pcbdoc.rs:25
  fn open_pcbdoc(path: &Path) -> Result<PcbDoc, String>
  // pcbdoc.rs:67
  pub fn cmd_overview(path: &Path) -> Result<PcbDocOverview, Box<dyn std::error::Error>>
  ```
- **Recommendation**: Pick `Box<dyn std::error::Error>` for the ops layer (it's a CLI boundary, heterogeneous errors are expected) and convert everything in one pass. Delete `open_schdoc` (String version), keep only the Box version, rename it to just `open_schdoc`. No users means no migration concern.

---

### 5. v1/v2 Parallel Type Hierarchies With No Bridge

- **Severity**: warning
- **Category**: Architecture — Parallel Type Hierarchies
- **Location(s)**:
  - `crates/altium-format/src/records/pcb/` (v1)
  - `crates/altium-format/src/v2/pcb/` (v2)
  - `crates/altium-format/src/records/sch/` (v1 schematic)
  - `crates/altium-format/src/v2/fields/` (v2 schematic)
- **Problem**: The v2 module was introduced to fix "several critical bugs" (v2/mod.rs:3), including a coordinate system bug (v1 uses 10,000 units/mil, v2 uses 100,000). Both systems are public, both are maintained, and there's no `From`/`Into` conversion between them. The lib.rs docs don't mention v2 exists. For PCB types specifically, `PcbPad` exists in both `records/pcb/pad.rs` and `v2/pcb/pad.rs` with completely different field structures.
- **Evidence**:
  ```rust
  // v2/mod.rs:1-6 — Acknowledges the parallel system
  //! V2 Altium format implementation ported from decompiled C#.
  //! This module runs in parallel with the existing (v1) code and fixes
  //! several critical bugs...

  // v2/coord.rs — Different coordinate scale
  // V2Coord: 100,000 units/mil
  // vs types/coord.rs Coord: 10,000 units/mil
  ```
- **Recommendation**: Since there are no external users, pick a direction. If v2 is correct (and it likely is, given it was ported from decompiled C# with bug fixes), migrate v1 consumers to v2 types and delete v1. If both must coexist temporarily, at minimum: (1) document which to use when in lib.rs, (2) provide `From<V2Coord> for Coord` conversions, (3) mark v1 types as `#[doc(hidden)]` if they're being phased out.

---

### 6. ops/ Modules Are Too Large and Mix Concerns

- **Severity**: warning
- **Category**: Separation of Concerns / KISS
- **Location(s)**:
  - `crates/altium-format/src/ops/schlib.rs` — 3,541 lines
  - `crates/altium-format/src/ops/pcblib.rs` — 3,475 lines
  - `crates/altium-format/src/ops/pcbdoc.rs` — 2,688 lines
  - `crates/altium-format/src/ops/output.rs` — 1,588 lines (172 pub structs)
- **Problem**: Each ops module is a monolithic file where every `cmd_*` function opens a file, does business logic, and constructs output structs — all in one function. This makes it impossible to test business logic without touching the filesystem. The codebase acknowledges this explicitly:
  ```rust
  // ops/schlib.rs:8
  // cmd_* functions mix presentation and business logic; separation punted until
  // usage patterns clarify abstraction boundaries (premature abstraction risk)
  ```
  This comment was reasonable at 500 lines. At 3,500 lines, the usage patterns have clarified.

  `output.rs` has 172 `pub struct` definitions — essentially a flat grab-bag of every output type for every command. Many share fields (name, description, pin_count) but aren't composed from shared base types.
- **Recommendation**: For each `cmd_foo()`, split into: (1) a pure function `analyze_foo(data: &SchLib) -> FooResult` that takes already-loaded data and returns a result, and (2) a thin `cmd_foo(path)` wrapper that opens the file and calls the pure function. This immediately makes the logic testable. Group output structs by domain (schlib output types, pcbdoc output types) into submodules.

---

### 7. 172 Output Structs With No Display Implementations

- **Severity**: warning
- **Category**: Separation of Concerns
- **Location(s)**:
  - `crates/altium-format/src/ops/output.rs` — 1,588 lines, 172 pub structs
  - `crates/altium-cli/src/output.rs` — TextFormat trait
- **Problem**: The library crate (`altium-format`) defines 172 output structs that are pure data bags used for CLI presentation. These are `Serialize + Deserialize + Clone` but have no `Display` or formatting logic. The CLI crate defines a `TextFormat` trait but uses it via `TextWrapper` which just serializes to JSON and prints. This means the "text mode" output of the CLI is actually JSON with labels — not human-readable formatted text.

  These output types also live in the library crate despite being CLI presentation concerns. A library user who just wants to parse Altium files gets 172 output structs they don't need.
- **Recommendation**: Move output types to the CLI crate. They're presentation layer types, not domain types. If you need some shared output types for the library API, keep those minimal and separate from CLI-specific formatting.

---

### 8. Dead Code and "Reserved for Future" Patterns

- **Severity**: note
- **Category**: KISS / Dead Code
- **Location(s)**:
  - `crates/altium-format-derive/src/record.rs:646-747` — 100 lines of dead code
  - `crates/altium-format/src/edit/library.rs:560-570` — deprecated + dead
  - `crates/altium-format/src/footprint/measure.rs:132-170` — "Reserved for future"
  - `crates/altium-format/src/query/view.rs:262-286` — multiple "Reserved for future" fields
  - `crates/altium-format/src/ops/schlib.rs:754-759` — "Reserved for future"
  - `crates/altium-cli/src/output.rs:45-72` — dead `print_json_as_text`
- **Problem**: ~200 lines of code that is annotated as dead/reserved across the codebase. Each carries a `#[allow(dead_code)]` suppression with a comment. This adds noise to the codebase and suppresses useful compiler warnings.
- **Evidence**:
  ```rust
  #[allow(dead_code)] // Reserved for future pad collision detection
  fn pad_rect(pad: &PcbPad) -> Rect<f64> { ... }

  #[allow(dead_code)] // Reserved for future hierarchical record queries
  tree: RecordTree<SchRecord>,

  #[allow(dead_code)] // Reserved for future ERC checks based on pin electrical types
  electrical_type: ElectricalType,
  ```
- **Recommendation**: Delete it all. Git remembers. When you actually need pad collision detection, you'll write better code informed by the actual requirements rather than a guess from today.

---

### 9. `open_*` Helper Functions Duplicated Per Module

- **Severity**: note
- **Category**: DRY Violation
- **Location(s)**:
  - `ops/schlib.rs:32-35`, `ops/pcblib.rs:36-39`, `ops/pcbdoc.rs:25-28`, `ops/schdoc.rs:33-42`, `ops/intlib.rs:16-19`, `ops/prjpcb.rs:18-20`
- **Problem**: Every ops module defines its own 3-line `open_*` function that does `File::open → BufReader → Type::open`. The pattern is identical; only the types differ.
- **Evidence**:
  ```rust
  // schlib.rs:32
  fn open_schlib(path: &Path) -> Result<SchLib, Box<dyn std::error::Error>> {
      let file = File::open(path)?; Ok(SchLib::open(BufReader::new(file))?)
  }
  // pcblib.rs:36
  fn open_pcblib(path: &Path) -> Result<PcbLib, Box<dyn std::error::Error>> {
      let file = File::open(path)?; Ok(PcbLib::open(BufReader::new(file))?)
  }
  // pcbdoc.rs:25 — different error type!
  fn open_pcbdoc(path: &Path) -> Result<PcbDoc, String> {
      let file = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
      PcbDoc::open(BufReader::new(file)).map_err(|e| format!("Error parsing PcbDoc: {:?}", e))
  }
  ```
  Note the error type inconsistency: some use `Box<dyn Error>`, one uses `String`.
- **Recommendation**: Add an `open_file(path)` associated function to each IO type (SchLib, PcbLib, etc.) that handles the `File::open → BufReader → parse` pattern. Then remove all the `open_*` helpers from ops modules. `PcbDoc` already has `open_file` — extend the pattern to all types.

---

### 10. `electrical_type_name` Reimplements Display

- **Severity**: note
- **Category**: Manual Reimplementation
- **Location(s)**:
  - `crates/altium-format/src/ops/schlib.rs:38-49`
  - `PinElectricalType` enum at `crates/altium-format/src/records/sch/common.rs:114`
- **Problem**: `electrical_type_name()` manually matches every variant of `PinElectricalType` to return a display string. This is what `Display` (or at minimum, a `name()` method on the enum) is for. If a new variant is added, this function must be updated separately.
- **Recommendation**: Implement `Display` for `PinElectricalType` or add a `name(&self) -> &'static str` method. Delete the free function.

---

### 11. `PcbObjectId::from_byte` Could Use TryFrom

- **Severity**: note
- **Category**: Rust Antipatterns — Ignoring the Type System
- **Location(s)**:
  - `crates/altium-format/src/records/pcb/primitive.rs:69-99`
- **Problem**: `from_byte` silently returns `PcbObjectId::None` for unknown values. This hides parsing errors — if a file contains an unexpected object ID, the code silently treats it as None rather than surfacing the issue.
- **Evidence**:
  ```rust
  pub fn from_byte(value: u8) -> Self {
      match value {
          0 => PcbObjectId::None,
          // ...
          _ => PcbObjectId::None,  // Silent data loss
      }
  }
  ```
- **Recommendation**: Implement `TryFrom<u8>` that returns `Result<Self, u8>` for the unknown case. Let the caller decide whether to ignore, warn, or error on unknown object IDs.

---

### 12. `unsafe` transmute for Enum Conversion

- **Severity**: note
- **Category**: Rust Antipatterns
- **Location(s)**:
  - `crates/altium-format/src/v2/types.rs` — ~45 occurrences
  - `crates/altium-format/src/v2/pcb/enums.rs` — 3 occurrences
- **Problem**: Every `from_u8` method on v2 enums uses `unsafe { std::mem::transmute(v) }`. While the bounds check before the transmute makes this sound, it's unnecessarily unsafe. The `#[repr(u8)]` enums with contiguous discriminants could use a match (the compiler optimizes it identically) or a crate like `num_enum` which provides safe `TryFrom` derives.
- **Evidence**:
  ```rust
  pub fn from_u8(v: u8) -> Option<Self> {
      if v <= 115 {
          Some(unsafe { std::mem::transmute(v) })
      } else {
          None
      }
  }
  ```
  The bound `115` must be manually kept in sync with the enum — if a variant is added or removed, this is a soundness bug.
- **Recommendation**: Replace with match statements or derive `TryFrom<u8>` via `num_enum`. The performance is identical, and it eliminates the soundness maintenance burden.

---

## Systemic Patterns

### Pattern 1: Copy-Paste Over Abstraction

The codebase has a tendency to copy small functions rather than extracting shared utilities. `record_type_name` (3 copies), `parse_coord` (4 copies), `open_*` (6 copies), and `categorize_*` (2 copies with similar structure) all show this pattern. Each copy diverges slightly over time, introducing subtle inconsistencies.

### Pattern 2: "Premature Abstraction Avoidance" Taken Too Far

Several comments reference avoiding premature abstraction (e.g., the ops/schlib.rs:8 comment about "separation punted until usage patterns clarify"). This was a reasonable stance early on, but at 3,500-line files with 172 output structs, the patterns have clarified. The pendulum has swung too far toward concrete, duplicated code.

### Pattern 3: Phantom Backwards Compatibility

The `#[deprecated]` annotations, dual `open_schdoc`/`open_schdoc_boxed` functions, `_det: &mut ()` parameters, and "reserved for future" dead code all suggest a codebase being maintained as if it has external users. At v0.1.x with no published dependents, this is pure overhead.

### Pattern 4: Presentation Types in the Library Crate

172 output structs in `ops/output.rs` and the entire `ops/` module tree are CLI presentation concerns living in the library crate. A library consumer importing `altium-format` to parse files gets all of this in their dependency.

## Architecture Assessment

### What Works Well

1. **Three-crate architecture** is clean: derive → format → cli with no circular dependencies.
2. **Derive macro system** (`AltiumRecord`, `AltiumBase`, `AltiumEnum`) is well-designed and eliminates significant boilerplate for record serialization.
3. **Non-destructive editing** via `UnknownFields` preservation is a good design decision for a format library that doesn't understand every field.
4. **Query engine** with CSS-like selectors (parser → engine → executor) has clean separation and is a useful abstraction.
5. **Template system** with JSON Schema support is well-thought-out for programmatic/LLM-driven component creation.
6. **CLI structure** is consistent and well-organized with clap derive.

### What Needs Work

1. **v1/v2 split** is the biggest architectural concern. Two coordinate systems (10K vs 100K units/mil) is not "running in parallel" — it's a correctness issue. The codebase needs to pick one and migrate.
2. **ops/ layer** has grown beyond its design. It was meant to be thin command wrappers but now contains substantial business logic, output formatting, and I/O — all interleaved.
3. **Public API surface** is too broad. Everything is `pub mod` with re-exports. There's no distinction between "this is the library API" and "this is internal implementation."
4. **Test coverage** is weak for the surface area. Roundtrip tests exist for v2 serialization, but ops/ (16K lines), edit/ (5.5K lines), and CLI commands (5.8K lines) have zero tests.

### Dependency Graph (Clean)

```
altium-format-derive (proc macros only: proc-macro2, quote, syn)
     ↓
altium-format (core: 20 deps including cfb, serde, png, resvg, geo)
     ↓
altium-cli (thin: clap, serde_json, schemars)
```

No cycles. No unnecessary coupling. The derive crate properly avoids runtime dependencies.

## Recommendations (Prioritized)

1. **Delete all backwards-compat dead weight**: Remove deprecated functions, `#[allow(dead_code)]` blocks, dual error-type open functions, and `_det: &mut ()` parameters. This is a single commit that removes ~300 lines of noise.

2. **Unify error handling in ops/**: Change all ops functions to return `Result<T, Box<dyn std::error::Error>>`. Delete `open_schdoc` (String version). One error type, one pattern.

3. **Consolidate duplicated functions**: Replace 3 copies of `record_type_name` with the existing method on `SchRecord`. Replace 4 copies of `parse_coord` with a single canonical implementation. Move `open_*` patterns to associated functions on the IO types.

4. **Decide the v1/v2 future**: Either migrate to v2 types throughout (recommended — v2 fixes real bugs), or document a clear boundary ("v1 for reading, v2 for writing"). The current state where both exist without documentation or conversion is unsustainable.

5. **Split ops/ modules**: Extract pure business logic functions that take loaded data (`&SchLib`, `&PcbDoc`) and return results, separate from the `cmd_*` wrappers that handle file I/O. This enables testing without filesystem access.

6. **Move output types to CLI crate**: The 172 output structs in `ops/output.rs` are CLI presentation types. Move them to `altium-cli` where they belong. Keep the library focused on file format types.

7. **Add tests for ops/ and edit/ layers**: These modules contain the most complex logic (query execution, edit sessions, component manipulation) but have zero test coverage. At minimum, add tests for the pure logic once it's separated from I/O (see recommendation 5).

8. **Replace unsafe transmute with safe conversion**: Use match statements or `num_enum` for the v2 enum `from_u8` methods. Eliminates ~48 unsafe blocks.
