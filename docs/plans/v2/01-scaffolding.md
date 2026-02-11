# Phase 0: Scaffolding & v1 Removal

**Agents: 1** (must complete before all other phases)
**Estimated scope: ~200 lines changed**

## Objective

Remove v1 modules from the module hierarchy so builds break immediately. Create the v2 module skeleton that all subsequent phases will populate.

## Prerequisites

None — this is the first phase.

## Tasks

### 0.1: Strip lib.rs to v2-only exports

**File: `crates/altium-format/src/lib.rs`**

Replace the entire file with a minimal skeleton that only exports the new v2 module and essential shared modules:

```rust
//! Altium file format library for Rust — v2 API.

pub mod error;
pub mod format;
pub mod v2;

pub use error::{AltiumError, Result};
```

**What this removes:**
- `pub mod dump;`
- `pub mod io;` (v1 low-level IO)
- `pub mod ops;` (v1 operations — will be rebuilt as `v2::ops`)
- `pub mod traits;` (v1 conversion traits)
- `pub mod types;` (v1 shared types — ParameterCollection etc. moves to v2)
- `pub mod templates;` (if exported — JSON template system replaced by code templates)
- All re-exports of v2 types, v1 types, derive macros

**NOTE:** Do NOT delete the source files for these modules. Only remove them from `lib.rs`. They remain as reference.

### 0.2: Create v2 module skeleton

**File: `crates/altium-format/src/v2/mod.rs`**

Replace the current v2/mod.rs with the new skeleton. The current v2/ files remain on disk as reference but are no longer part of the module tree.

```rust
//! V2 Altium format API — backing-store architecture.

// Foundation (Phase 1)
pub mod backing_store;
pub mod coord;
pub mod traits;
pub mod newtypes;
pub mod binary_helpers;

// Record types (Phase 3) — populated by macro-generated types
pub mod records;

// View types (Phase 4)
pub mod views;

// Document types (Phase 4)
pub mod documents;

// Query language (Phase 5)
pub mod query;

// Templates & builders (Phase 6)
pub mod templates;
pub mod builders;

// CLI operations (Phase 7)
pub mod ops;
```

### 0.3: Create empty submodule files

Create each submodule with a minimal placeholder so the skeleton compiles:

**Files to create** (each with `//! TODO: Phase N` comment):

```
crates/altium-format/src/v2/backing_store.rs      # //! Phase 1B
crates/altium-format/src/v2/coord.rs               # RENAME existing (see below)
crates/altium-format/src/v2/traits.rs              # //! Phase 1C
crates/altium-format/src/v2/newtypes.rs            # //! Phase 1D
crates/altium-format/src/v2/binary_helpers.rs      # //! Phase 1E
crates/altium-format/src/v2/records/mod.rs         # //! Phase 3
crates/altium-format/src/v2/views/mod.rs           # //! Phase 4D
crates/altium-format/src/v2/documents/mod.rs       # //! Phase 4
crates/altium-format/src/v2/query/mod.rs           # //! Phase 5
crates/altium-format/src/v2/templates.rs           # //! Phase 6
crates/altium-format/src/v2/builders.rs            # //! Phase 6
crates/altium-format/src/v2/ops/mod.rs             # //! Phase 7
```

### 0.4: Relocate existing v2 files to reference directory

Move current v2/ source files to a reference location so they don't conflict with the new module structure:

```bash
mkdir -p crates/altium-format/src/_v2_reference
mv crates/altium-format/src/v2/fields crates/altium-format/src/_v2_reference/
mv crates/altium-format/src/v2/io crates/altium-format/src/_v2_reference/
mv crates/altium-format/src/v2/serializer crates/altium-format/src/_v2_reference/
mv crates/altium-format/src/v2/pcb crates/altium-format/src/_v2_reference/
mv crates/altium-format/src/v2/types.rs crates/altium-format/src/_v2_reference/
mv crates/altium-format/src/v2/consts.rs crates/altium-format/src/_v2_reference/
mv crates/altium-format/src/v2/coord.rs crates/altium-format/src/_v2_reference/
```

The new `v2/coord.rs` will be created fresh by Phase 1A.

### 0.5: Move ParameterCollection to v2

Copy `types/parameters.rs` into the v2 module structure since it's needed by the backing store. This is the ONE v1 type that gets directly reused (it's already order-preserving with IndexMap):

```bash
cp crates/altium-format/src/types/parameters.rs crates/altium-format/src/v2/parameters.rs
```

Add `pub mod parameters;` to `v2/mod.rs`.

Clean up the copied file to remove any imports from `crate::types::*` or `crate::traits::*` — make it self-contained within v2. It should depend only on `crate::error`.

### 0.6: Add pest dependency

**File: `crates/altium-format/Cargo.toml`**

Add:
```toml
pest = "2.7"
pest_derive = "2.7"
```

### 0.7: Fix CLI crate to compile (stub)

**File: `crates/altium-cli/src/main.rs`**

The CLI crate will fail to compile because it depends on v1 types. Create a minimal stub that compiles but does nothing useful:

```rust
fn main() {
    eprintln!("altium-cli: v2 refactoring in progress — commands temporarily disabled");
    std::process::exit(1);
}
```

Comment out or remove the `commands/` module imports and clap setup temporarily. The CLI will be rebuilt in Phase 7.

### 0.8: Verify builds

Run:
```bash
cargo check --workspace
```

The workspace should compile with just the skeleton modules (all empty). Integration tests will fail — that's expected.

## Acceptance Criteria

- [ ] `crates/altium-format/src/lib.rs` only exports `error`, `format`, and `v2`
- [ ] No v1 modules (`types/`, `traits/`, `io/`, `ops/`, `records/`, `dump/`, `templates/`) are in the module tree
- [ ] v1 source files still exist on disk (NOT deleted)
- [ ] Current v2 files moved to `_v2_reference/` directory
- [ ] New v2 module skeleton with empty submodules
- [ ] `ParameterCollection` copied to v2 and self-contained
- [ ] `pest` and `pest_derive` added to Cargo.toml
- [ ] `cargo check --workspace` passes (everything compiles, even if empty)
- [ ] CLI crate compiles (as a stub)

## Output

After this phase, the module tree looks like:

```
lib.rs → error, format, v2
v2/
  mod.rs (skeleton)
  parameters.rs (from types/parameters.rs)
  backing_store.rs (empty)
  coord.rs (empty)
  traits.rs (empty)
  newtypes.rs (empty)
  binary_helpers.rs (empty)
  records/mod.rs (empty)
  views/mod.rs (empty)
  documents/mod.rs (empty)
  query/mod.rs (empty)
  templates.rs (empty)
  builders.rs (empty)
  ops/mod.rs (empty)
```

All builds pass. All tests fail. This is correct.
