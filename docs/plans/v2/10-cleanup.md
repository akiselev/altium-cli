# Phase 9: Cleanup & Final Validation

**Agents: 1** (single agent — this is the final sweep)
**Blocked by: Phase 8 (all tests passing)**

## Objective

Remove all v1 source files, remove old derive macros, verify everything still compiles and tests pass. This is the point of no return.

## Prerequisites

ALL of the following must be true before starting this phase:

- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes (including `--ignored` for fixture tests)
- [ ] JSON roundtrip tests pass for SchLib and PcbLib
- [ ] CFB roundtrip tests pass for SchLib and PcbLib
- [ ] CLI commands produce correct output
- [ ] No code imports from v1 modules

## Tasks

### 9.1: Remove v1 Source Files

Delete the following directories and files:

```bash
# v1 modules (already removed from lib.rs in Phase 0)
rm -rf crates/altium-format/src/types/
rm -rf crates/altium-format/src/traits/
rm -rf crates/altium-format/src/io/
rm -rf crates/altium-format/src/ops/
rm -rf crates/altium-format/src/records/
rm -rf crates/altium-format/src/templates/
rm crates/altium-format/src/dump.rs

# v2 reference files (no longer needed)
rm -rf crates/altium-format/src/_v2_reference/
```

### 9.2: Remove Old Derive Macros

In `crates/altium-format-derive/src/`:

- Remove `record.rs` (old `#[derive(AltiumRecord)]`)
- Remove `base.rs` (old `#[derive(AltiumBase)]`)
- Remove `enum_derive.rs` (old `#[derive(AltiumEnum)]`)
- Update `lib.rs` to only export the new `#[altium_record]` and `#[altium_enum]` attribute macros
- Clean up `attrs.rs` to only have new attribute parsing

Verify the old derive macros are not referenced anywhere:

```bash
grep -r "derive(AltiumRecord)" crates/ --include="*.rs"
grep -r "derive(AltiumBase)" crates/ --include="*.rs"
grep -r "derive(AltiumEnum)" crates/ --include="*.rs"
```

All should return empty.

### 9.3: Clean Up Cargo.toml

Remove any dependencies that were only used by v1 code and are no longer needed:

**Review `crates/altium-format/Cargo.toml`:**
- `geo` — check if still used (geometry types)
- `png`, `resvg` — check if still used (image rendering)
- `regex` — check if still used (may be replaced by pest for queries)
- `base64` — check if still used

Keep everything that's still imported. Only remove dependencies with zero imports.

### 9.4: Clean Up Re-exports in lib.rs

Final `lib.rs` should be minimal:

```rust
//! Altium file format library for Rust.
//!
//! # Usage
//!
//! ```ignore
//! use altium_format::v2::documents::schlib::SchLib;
//! use altium_format::v2::views::SchComponent;
//!
//! let mut lib = SchLib::open_file("library.SchLib")?;
//! lib.query::<SchComponent>("U1")?.with_mut(|comp| {
//!     comp.set_description("Updated");
//! });
//! lib.save_file("library.SchLib")?;
//! ```

pub mod error;
pub mod format;
pub mod v2;

pub use error::{AltiumError, Result};

// Re-export the most commonly used types for convenience
pub use v2::documents::{SchLib, SchDoc, PcbLib};
pub use v2::views::{SchComponent, SchPin, PcbFootprint, PcbPad};
pub use v2::coord::{SchCoord, PcbCoord};
```

### 9.5: Final Validation

Run the complete test suite:

```bash
# All unit tests
cargo test --workspace

# All integration tests (including ignored fixture-dependent ones)
cargo test --workspace -- --ignored

# Build the CLI binary
cargo build --workspace

# Smoke test CLI
./target/debug/altium-cli schlib list ../../Synthiam.SchLib
./target/debug/altium-cli schlib export ../../Synthiam.SchLib
./target/debug/altium-cli pcblib list ../../Synthiam.PcbLib
```

### 9.6: Verify No Dead Code

```bash
# Check for unused imports/code
cargo clippy --workspace -- -W dead_code -W unused_imports

# Check that no test references removed v1 types
grep -r "crate::types::" crates/ --include="*.rs"
grep -r "crate::traits::" crates/ --include="*.rs"
grep -r "crate::io::" crates/ --include="*.rs"
grep -r "crate::ops::" crates/ --include="*.rs"
```

All should return empty (only `crate::v2::` imports should remain).

### 9.7: Update Documentation

- Update `CLAUDE.md` with new module structure
- Update crate-level doc comments in `lib.rs`
- Remove any references to v1 API in doc comments

## Acceptance Criteria (Definition of Done — Full Refactoring)

From `docs/v2-plan.md`:

- [ ] 1. All record types use backing-store access — no runtime typed fields
- [ ] 2. All getters/setters use proper domain newtypes, coordinates, enums, bitflags
- [ ] 3. Param types handle their own serialization via `ParamCodec` (single key)
- [ ] 4. Core types have zero implicit defaults — new records from template functions only
- [ ] 5. `UnknownFields` type is removed entirely
- [ ] 6. `SchCoord` (100K/mil) and `PcbCoord` (10K/mil) are separate types
- [ ] 7. Boundary `Measurement<U>` type for unit conversions
- [ ] 8. Hierarchical view types Deref/DerefMut to record types with dirty tracking
- [ ] 9. `ComponentGroup` separates component from children for split-borrow
- [ ] 10. Query API: `query::<T>(q)` and `query_all::<T>(q)` work
- [ ] 11. `QueryHandle`/`ChildHandle` have `with_mut`; `ChildKey` for passing handles around
- [ ] 12. `Designator` is a single newtype for concrete and template forms
- [ ] 13. E2E tests rebuild from JSON using templates
- [ ] 14. diff-ole.py has exit codes and strict/semantic modes
- [ ] 15. Tests assert functional behavior (byte identity, patch locality, invariants)
- [ ] 16. CLI uses new core API
- [ ] 17. AQL parser uses pest (pattern selectors + attribute selectors)
- [ ] 18. `#[altium_enum]` generates `AltiumEnum` impl with blanket `ParamCodec`
- [ ] 19. v1 module hierarchy fully removed — no deprecated code remains

## Output

After this phase:
- Zero v1 code in the module tree
- Zero v1 source files on disk
- All tests pass
- CLI works
- The v2 architecture from `docs/v2-plan.md` is fully implemented
