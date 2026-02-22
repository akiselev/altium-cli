# Low-Level API Implementation Plan

## Overview

Implement the 5-layer parsing stack described in `docs/designs/low-level-api.md` inside
`crates/altium-format`. The approach is hybrid: build shared infrastructure (Layers 1-4
+ error types) first, then vertical-slice through SchLib as the first document format.
SchLib is the entry point of the EE workflow and exercises all key patterns: block-framed
streams, mixed text/binary dispatch, embedded object envelopes, and pin sidecar merging.

Layers 1-2 are designed to fail fast on unimplemented streams, and Layer 4's exhaustion
checks fail on unknown parameters. This drives the red/green development loop: run
`altium validate` against real SchLib files, fix the first error, repeat.

Stream processing order within SchLib follows the design doc: FileHeader first, then
Storage, then per-component Data streams with pin sidecars.

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
| --- | --- |
| SchLib first, not SchDoc | SchLib is first in the EE design workflow (create symbols before schematics) -> exercises all key patterns (mixed text/binary blocks, pin sidecars, embedded objects) -> forces us to implement the most complex sidecar system early -> SchDoc reuses the same Layer 3-5 infrastructure afterward |
| Hybrid approach (shared infra + vertical slice) | Pure bottom-up delays red/green feedback -> pure vertical-slice risks refactoring shared layers -> hybrid builds tested shared infrastructure once, then iterates per-format with fast feedback |
| `IndexMap` for ParameterCollection | Altium serializes parameters in a specific order -> round-trip fidelity requires preserving insertion order -> `IndexMap` provides O(1) lookup with insertion-order iteration -> already in Cargo.toml dependencies |
| Manual record implementations before derive macros | Pattern diversity across record types (indexed families, coord pairs, binary subrecords) -> building macros without understanding all patterns risks rework -> implementing 5-10 records manually reveals the complete attribute surface -> derive macros then codify proven patterns |
| RECORD=0 sentinel calls assert_exhausted before returning Ok(None) | SchLib/SchDoc Data streams use RECORD=0 as end-of-stream sentinel -> assert_exhausted is called before returning Ok(None) to catch any unknown params that appear alongside the sentinel -> this upholds the fail-fast invariant: if Altium ever adds params to the sentinel block, they are not silently dropped -> FileHeader block 0 (which also has RECORD=0) IS exhaustion-checked separately in the document loader |
| No skip_known for unimplemented streams | skip_known silently acknowledges data we don't parse -> violates fail-fast philosophy -> if a sidecar stream exists and we can't parse it, assert_all_consumed must error -> forces us to implement parsers before claiming success |
| parse_embedded_object_stream asserts exhaustion internally | Header params (RECORD, Weight) are consumed inside the function -> no caller ever needs the header ParameterCollection -> returning a live ParameterCollection would be misleading (it's always empty after exhaustion) -> return only `Vec<EmbeddedObject>` -> simpler API, single enforcement point |
| Binary bool encoding: 0x00 = false, non-zero = true | Delphi Boolean type is 1 byte where 0 = False and any non-zero = True -> Altium's Delphi codebase follows this convention -> strict 0x00/0x01 check would reject valid files that use non-0x01 true values -> non-zero = true is the safe choice |
| Unknown component storage returns CfbError | Component storages without Data or Redirection sub-streams are malformed CFB structure -> `CfbError(format!("storage /{key}/ has neither Data nor Redirection stream"))` provides clear context -> no new error variant needed; CfbError already handles structural CFB anomalies |
| Pin sidecar index out-of-bounds returns Err, not panic | Sidecar entries come from external file data (untrusted input) -> out-of-bounds index is a malformed-file condition, not a programming error -> must return `Err(AltiumFormatError::InvalidEmbeddedObject(format!("pin index {idx} out of bounds (component has {len} pins)")))` -> preserves red/green CLI loop with clean error messages |
| `assert_exhausted` at dispatch boundary, not inside `FromParams`/`FromBinary` | Base types compose via flatten (SchPrimitiveBase fields + record-specific fields share one ParameterCollection) -> exhaustion check inside base type would reject record-specific fields -> single enforcement point at dispatcher is simpler and correct |
| Windows-1252 as default text encoding, not UTF-8 | Altium parameter strings are Windows-1252 encoded -> `encoding_rs` crate handles conversion -> `%UTF8%` prefix on key names signals UTF-8 values as the exception, not the rule |
| `pub(crate)` for all parsing machinery | Design doc mandates privacy -> only `SchLib`, `SchDoc`, etc. and their record types are public -> Layers 1-4 are implementation details that must not leak to `altium-format-ops` |
| Separate `CfbDocument` and `TrackedCfbDocument` | Separation of concerns: Layer 1 handles CFB I/O, Layer 2 adds consumption tracking -> TrackedCfbDocument composes CfbDocument rather than inheriting -> each layer is independently testable |
| `Cursor<Vec<u8>>` for CFB, not file handle | Altium files are typically <100 MB -> reading entire file into memory simplifies lifetime management -> `cfb` crate works directly with `Cursor<Vec<u8>>` |
| Block header: bits 0-23 = size, bits 24-31 = flags | Altium's block header packs size and format discriminant into a single i32 -> 0x00 = text (pipe-delimited params), 0x01 = binary (packed struct) -> masking with `0x00FF_FFFF` extracts size |
| Unescape 0x8E and 0xA6 as pipe in parameter values | Altium encodes literal `\|` inside parameter values as byte 0x8E (142); double 0x8E 0x8E encodes literal 0x8E -> 0xA6 (broken bar ¦) is alternate pipe escape in ASCII format -> `unescape_param_value` must post-process after pipe-splitting to restore embedded pipes -> verified in `StrUtils.ReplaceSpecialDelimiterChars` and `ProcessMBCSString` in decompiled .NET source |
| Pin sidecar length prefixes are signed i32 LE | Decompiled .NET uses `BitConverter.ToInt32` / `BitConverter.GetBytes(int)` for all sidecar length prefixes -> NOT u32 -> negative lengths indicate malformed data and should error |
| GraphicallyLocked must be consumed but ignored | PinConglomerate bit 6 is written to files but Altium hardcodes it to `false` on import (`FileFormatV5.cs`) -> must be read and discarded (not skipped) so `assert_exhausted` passes -> same pattern as other write-only fields |
| Inverted boolean fields use negated semantics | 9 fields use `!value` pattern on import (e.g., `IsNotAccesible` -> `SetIsAccessible(!value)`) -> `FromParams` / `FromBinary` must negate the stored value -> verified in decompiled .NET `FileFormatV5.cs` |
| BinaryReader provides `read_real48()` for 6-byte Borland Turbo Pascal Real | Binary-mode blocks store angles (StartAngle, EndAngle) as 6-byte Real48 -> `Real48.cs` in decompiled .NET provides conversion algorithm -> IEEE f64 equivalent: extract 8-bit exponent (byte 0), 40-bit mantissa (bytes 1-5), sign bit (MSB of byte 5), convert to f64 biased exponent |
| Import constants from `altium_format_types::constants` | `altium-format` already depends on `altium-format-types` in Cargo.toml -> stream names, parameter keys, instruction bytes, and unit constants should be imported rather than hardcoded -> keeps a single source of truth for values reverse-engineered from Altium's .NET assemblies |

### Rejected Alternatives

| Alternative | Why Rejected |
| --- | --- |
| SchDoc as first format | Simpler stream structure but lacks binary blocks and pin sidecars -> would not exercise the full Layer 3-5 surface -> SchLib forces us to implement everything |
| Single monolithic parser module | Violates the 5-layer design -> makes testing individual layers impossible -> makes it harder to reason about exhaustion boundaries |
| Derive macros first | Without manual implementations, we cannot verify the attribute surface covers all patterns -> risk of building macros that miss indexed families, coord pairs, or binary subrecords |
| `HashMap` for ParameterCollection | Loses insertion order -> serialized output would differ from Altium's -> user explicitly requires ordered map for round-trip fidelity |
| `BTreeMap` for ParameterCollection | Alphabetical order does not match Altium's insertion order -> same round-trip problem as HashMap |
| `skip_known` for unimplemented sidecar streams | Allows M9 to pass without implementing sidecars -> but silently acknowledges data we don't understand -> violates "fail fast, fail hard" -> if sidecars exist, we must parse them or fail |

### Constraints & Assumptions

- All parsing types are `pub(crate)` per CLAUDE.md privacy requirement
- All fallible operations return `Result<T, AltiumFormatError>` — no silent drops
- Test data available in `data/` directory: `BlankSchlibComponent.SchLib`, `LimeMicroAltiumLib_schLib.SchLib`, `Synthiam.SchLib`
- `cfb` 0.12.1, `indexmap` 2.13.0, `encoding_rs` 0.8.35, `flate2` 1.1.5 already in Cargo.toml
- Rust 2024 edition, MSRV 1.85
- `altium-format-derive` crate exists but is empty (1 line) — derive macros are out of scope for this plan
- Testing: property-based (proptest/quickcheck) for invariants like roundtrips and ordering; example-based for known Altium binary patterns; integration via real `.SchLib` files from `data/`
- `<default-conventions domain="testing">` applied: integration tests highest value, property-based preferred, unit tests sparingly

### Known Risks

| Risk | Mitigation | Anchor |
| --- | --- | --- |
| Real SchLib files may contain streams not documented in design doc | Layer 2 `assert_all_consumed()` will surface them immediately as `UnconsumedStreams` errors; investigate each unknown stream via ghidra/hex dump and implement its parser before proceeding | Design doc Layer 2 section |
| Parameter keys in real files may differ from design doc (case, spelling) | Case-insensitive key matching in ParameterCollection; first unknown key triggers `UnknownParams` error with the key name for debugging | Design doc Layer 4 "Key case" row |
| Block header flag values beyond 0x00/0x01 may exist | Strict match in `parse_blocks` — unknown flags are hard errors; investigate in ghidra if encountered | Design doc Layer 3 Format A |
| Pin sidecar ordering matters (PinWideText overwrites PinDesc) | Apply sidecars in exact documented order; test with real files that have both PinDesc and PinWideText | Design doc "Pin sidecar load order" |
| Binary blocks may contain 6-byte Real48 (Borland Turbo Pascal Real) for angles | BinaryReader includes `read_real48()` method; conversion algorithm verified against `Real48.cs` in decompiled .NET; if encountered, parse via exponent/mantissa extraction | Decompiled `Real48.cs` |

## Invisible Knowledge

### Architecture

```
                          SchLib::open(path)
                                │
                    ┌───────────┴───────────┐
                    │  Layer 2: TrackedCfb   │ ← assert_all_consumed() at end
                    │  Layer 1: CfbDocument  │ ← Cursor<Vec<u8>> over whole file
                    └───────────┬───────────┘
                                │
              ┌─────────────────┼─────────────────┐
              │                 │                  │
        /FileHeader        /Storage          /<Component>/
              │                 │              ┌───┴───┐
              ▼                 ▼              │       │
         Layer 3:          Layer 4:         Data    Sidecars
       parse_blocks    parse_embedded      │     (PinFrac,
              │         _object_stream     │      PinDesc,
              ▼                │           ▼      PinWideText,
         Layer 4:              ▼       Layer 3:   ...)
    ParameterCollection   EmbeddedObject  parse_blocks
              │                               │
              ▼                               ▼
         Layer 5:                        Layer 5:
    SchRecord::from_block           SchRecord::from_block
    (text → RECORD=N dispatch)      (text + binary dispatch)
    (binary → code dispatch)
```

### Data Flow

```
File bytes ──► cfb::CompoundFile ──► CfbDocument ──► TrackedCfbDocument
                                                          │
                                          read_stream/read_stream_optional
                                                          │
                                                     raw bytes
                                                          │
                              ┌────────────────┬──────────┴────────┐
                         parse_blocks    parse_embedded_object   (other L3)
                              │                    │
                         Vec<Block>         Vec<EmbeddedObject>
                              │                    │
                    ParameterCollection      BinaryReader/
                    or BinaryReader          ParameterCollection
                              │                    │
                    FromParams/FromBinary     merge into pins
                              │
                         SchRecord enum
```

### Why This Structure

- **5 layers exist because Altium has 5 distinct abstraction levels**: CFB container → stream enumeration → stream framing → field access → typed records. Collapsing layers would mix concerns (e.g., stream tracking with record parsing).
- **TrackedCfbDocument wraps CfbDocument** rather than extending it because consumption tracking is orthogonal to CFB I/O — tests for CfbDocument don't need tracking overhead.
- **ParameterCollection uses remove-on-read** (not get-and-mark) because it naturally prevents double-reads and makes the remaining-keys check trivial. The IndexMap shrinks as fields are consumed.
- **Embedded object envelope is in Layer 4, not Layer 3**, because the 0xD0 format appears _inside_ block payloads (flags=0x01 binary blocks). Layer 3 extracts blocks; Layer 4 interprets the block content.

### Invariants

- Every CFB stream must be consumed or explicitly acknowledged before `SchLib::open` returns
- Every parameter key must be consumed before the dispatcher returns a typed record
- Every byte in a binary record must be consumed before the dispatcher returns
- Pin sidecar streams must be applied in exact order: PinFrac → PinDesc → PinMiscData → PinTextData → PinWideText → PinSymbolLineWidth → PinPackageLength → PinPropagationDelay → PinFunctionData
- PinWideText overwrites PinDesc fields (Name, Designator, Description) — load order is authoritative
- ParameterCollection preserves insertion order (IndexMap) for deterministic serialization
- Case-insensitive key lookup; first occurrence wins for duplicates
- `RECORD=0` is sentinel, not a record type — calls `assert_exhausted` then returns `Ok(None)`, never `Err`
- `RECORD=254` means actual type is in `RECORDEX`
- Parameter values containing literal pipes are escaped: 0x8E → `|`, double 0x8E → literal 0x8E, 0xA6 (¦) → `|`, `[]` → `|`, `{}` → `=`; `unescape_param_value` must handle all four
- Write-only fields (e.g., `GraphicallyLocked` in PinConglomerate bit 6) must be consumed and discarded — skipping them would fail `assert_exhausted`
- Inverted boolean fields (e.g., `IsNotAccesible`) must be negated on import: stored `true` means the property is `false`

### Tradeoffs

- **Memory**: Loading entire file into `Vec<u8>` trades memory for simplicity. Files >100 MB would be problematic, but Altium files rarely exceed that.
- **Remove-on-read**: Destructive access means ParameterCollection cannot be re-read. This is by design — re-reading would indicate a bug in field consumption logic.
- **No derive macros in this plan**: Manual implementations are more verbose but necessary to discover the full attribute surface before codifying patterns.

## Milestones

### Milestone 1: Error Types Expansion

**Files**: `crates/altium-format/src/lib.rs`

**Flags**:
- `needs-rationale`: Each error variant maps to a specific layer's failure mode

**Requirements**:
- Expand `AltiumFormatError` enum with all variants from the design doc (Layer 1-5)
- Replace existing `InvalidParamValue(String)` with structured `InvalidParamValue { key, detail }`
- Replace `BinaryParsingError(String)` with `BinaryReadPastEnd { offset, needed, available }`
- Add: `CfbError(String)`, `StreamNotFound(String)`, `UnconsumedStreams { paths }`, `InvalidBlockHeader { offset, detail }`, `RecordCountMismatch { section, expected, actual }`, `MissingParam(String)`, `DecompressionError(String)`, `InvalidEmbeddedObject(String)`, `UnknownObjectId(u8)`, `UnknownBinaryCode(u8)`, `UnknownParams { keys }`, `UnexpectedTrailingData { offset, count }`
- Keep `Result<T>` type alias

**Acceptance Criteria**:
- `cargo check -p altium-format` passes
- Every error variant from the design doc's Error Types section exists
- All variants derive `Debug` and implement `Display` via `thiserror`

**Tests**: Skip — error type definitions are validated by the compiler.

**Code Intent**:
- Modify `AltiumFormatError` enum in `lib.rs`: replace the 4 existing variants with the full set from the design doc
- Keep `#[derive(Debug, thiserror::Error)]` and `pub type Result<T>`
- Structured fields on variants that carry context (key names, offsets, counts)

### Code Changes

```diff
--- a/crates/altium-format/src/lib.rs
+++ b/crates/altium-format/src/lib.rs
@@ -17,11 +17,56 @@ pub use schlib::SchLib;
 #[derive(Debug, thiserror::Error)]
 pub enum AltiumFormatError {
     #[error("IO error: {0}")]
     Io(#[from] std::io::Error),
-    #[error("Invalid parameter value: {0}")]
-    InvalidParamValue(String),
-    #[error("Unknown record type: {0}")]
-    UnknownRecordType(i32),
-    #[error("Binary parsing error: {0}")]
-    BinaryParsingError(String),
+    // Layer 1: CFB container errors
+    #[error("CFB error: {0}")]
+    CfbError(String),
+    #[error("Stream not found: {0}")]
+    StreamNotFound(String),
+    // Layer 2: Stream consumption tracking
+    #[error("Unconsumed streams: {paths:?}")]
+    UnconsumedStreams { paths: Vec<String> },
+    // Layer 3: Block stream framing
+    #[error("Invalid block header at offset {offset}: {detail}")]
+    InvalidBlockHeader { offset: usize, detail: String },
+    // Layer 4: Binary reader
+    #[error("Binary read past end at offset {offset}: needed {needed}, available {available}")]
+    BinaryReadPastEnd { offset: usize, needed: usize, available: usize },
+    #[error("Unexpected trailing data at offset {offset}: {count} bytes remaining")]
+    UnexpectedTrailingData { offset: usize, count: usize },
+    // Layer 4: Parameter collection
+    #[error("Invalid parameter value for key '{key}': {detail}")]
+    InvalidParamValue { key: String, detail: String },
+    #[error("Missing required parameter: {0}")]
+    MissingParam(String),
+    #[error("Unknown parameters: {keys:?}")]
+    UnknownParams { keys: Vec<String> },
+    // Layer 4: Embedded object envelope
+    #[error("Invalid embedded object: {0}")]
+    InvalidEmbeddedObject(String),
+    #[error("Unknown object ID: {0:#04x}")]
+    UnknownObjectId(u8),
+    #[error("Record count mismatch in {section}: expected {expected}, got {actual}")]
+    RecordCountMismatch { section: String, expected: usize, actual: usize },
+    // Layer 5: Record dispatch
+    #[error("Unknown record type: {0}")]
+    UnknownRecordType(i32),
+    #[error("Unknown binary code: {0:#04x}")]
+    UnknownBinaryCode(u8),
+    // Decompression
+    #[error("Decompression error: {0}")]
+    DecompressionError(String),
 }

 pub type Result<T> = std::result::Result<T, AltiumFormatError>;
```

---

### Milestone 2: Layer 1 — CfbDocument

**Files**: `crates/altium-format/src/cfb_document.rs`, `crates/altium-format/src/lib.rs`

**Flags**:
- `error-handling`: CFB errors must map cleanly to `AltiumFormatError::CfbError`

**Requirements**:
- `CfbDocument` struct wrapping `cfb::CompoundFile<Cursor<Vec<u8>>>`
- `open(path)`: read file into memory, open as CFB, return `Err(CfbError)` on invalid container
- `read_stream(path)`: read entire stream into `Vec<u8>`, `Err(StreamNotFound)` if missing
- `read_stream_optional(path)`: same but returns `Ok(None)` if missing
- `exists(path)`: check entry existence
- `list_entries(path)`: return `(Vec<String>, Vec<String>)` of (storages, streams)
- `enumerate_all_entries()`: recursively enumerate all entries as `HashSet<String>`
- All types `pub(crate)`
- Register module in `lib.rs`

**Acceptance Criteria**:
- `cargo check -p altium-format` passes
- Opens `data/BlankSchlibComponent.SchLib` without error
- `read_stream("/FileHeader")` returns bytes
- `read_stream("/NonExistent")` returns `Err(StreamNotFound)`
- `enumerate_all_entries()` returns all paths in the CFB

**Tests**:
- **Test files**: `crates/altium-format/tests/cfb_document.rs`
- **Test type**: integration (real `.SchLib` files)
- **Backing**: user-specified (real Altium files)
- **Scenarios**:
  - Normal: open BlankSchlibComponent.SchLib, read FileHeader stream
  - Normal: enumerate_all_entries returns expected set of paths
  - Edge: read_stream_optional on missing stream returns Ok(None)
  - Error: open non-existent path returns Io error
  - Error: read_stream on non-existent stream returns StreamNotFound

**Code Intent**:
- New file `cfb_document.rs`: `CfbDocument` struct with `cfb::CompoundFile<Cursor<Vec<u8>>>` inner field
- `open`: `std::fs::read(path)` → `Cursor::new(bytes)` → `cfb::CompoundFile::open` → map error to `CfbError`
- `read_stream`: `inner.open_stream(path)` → `read_to_end` → map `cfb` error. Check exists first for `StreamNotFound`.
- `list_entries`: iterate `inner.read_storage(path)` → partition by `is_stream` vs `is_storage`
- `enumerate_all_entries`: recursive walk from root, collect all paths
- Add `mod cfb_document;` to `lib.rs` (private module, no `pub use`)

### Code Changes

New file `crates/altium-format/src/cfb_document.rs`:

```diff
--- /dev/null
+++ b/crates/altium-format/src/cfb_document.rs
@@ -0,0 +1,80 @@
+//! Layer 1 of the 5-layer parsing stack: raw CFB container I/O.
+//! Wraps `cfb::CompoundFile` with error mapping to `AltiumFormatError`.
+//! Holds no consumption state — see `TrackedCfbDocument` for stream tracking.
+use std::collections::HashSet;
+use std::io::{Cursor, Read};
+use std::path::Path;
+
+use crate::{AltiumFormatError, Result};
+
+pub(crate) struct CfbDocument {
+    inner: cfb::CompoundFile<Cursor<Vec<u8>>>,
+}
+
+impl CfbDocument {
+    // Reads the file at `path` entirely into memory and opens it as a CFB container.
+    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self> {
+        let bytes = std::fs::read(path)?;
+        let cursor = Cursor::new(bytes);
+        let inner = cfb::CompoundFile::open(cursor)
+            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
+        Ok(Self { inner })
+    }
+
+    // Reads the entire stream at `path` into a Vec<u8>. Returns StreamNotFound if absent.
+    pub(crate) fn read_stream(&mut self, path: &str) -> Result<Vec<u8>> {
+        if !self.inner.exists(path) {
+            return Err(AltiumFormatError::StreamNotFound(path.to_owned()));
+        }
+        let mut stream = self
+            .inner
+            .open_stream(path)
+            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
+        let mut buf = Vec::new();
+        stream.read_to_end(&mut buf)?;
+        Ok(buf)
+    }
+
+    // Reads the stream at `path` if it exists; returns Ok(None) when absent.
+    pub(crate) fn read_stream_optional(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
+        if !self.inner.exists(path) {
+            return Ok(None);
+        }
+        self.read_stream(path).map(Some)
+    }
+
+    // Returns true if the entry at `path` exists in the CFB container.
+    pub(crate) fn exists(&self, path: &str) -> bool {
+        self.inner.exists(path)
+    }
+
+    // Returns (storages, streams) for the given storage path.
+    pub(crate) fn list_entries(&mut self, path: &str) -> Result<(Vec<String>, Vec<String>)> {
+        let entries = self
+            .inner
+            .read_storage(path)
+            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
+        let mut storages = Vec::new();
+        let mut streams = Vec::new();
+        for entry in entries {
+            let name = entry.name().to_owned();
+            if entry.is_storage() {
+                storages.push(name);
+            } else {
+                streams.push(name);
+            }
+        }
+        Ok((storages, streams))
+    }
+
+    // Walks all CFB entries recursively from root and returns their full paths.
+    pub(crate) fn enumerate_all_entries(&mut self) -> Result<HashSet<String>> {
+        let mut result = HashSet::new();
+        self.enumerate_recursive("/", &mut result)?;
+        Ok(result)
+    }
+
+    // Recursively walks all CFB entries under `path`, appending paths to `out`.
+    fn enumerate_recursive(&mut self, path: &str, out: &mut HashSet<String>) -> Result<()> {
+        let entries = self
+            .inner
+            .read_storage(path)
+            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
+        let children: Vec<(String, bool)> = entries
+            .map(|e| (e.path().display().to_string().trim_end_matches('/').to_owned(), e.is_storage()))
+            .collect();
+        for (child_path, is_storage) in children {
+            out.insert(child_path.clone());
+            if is_storage {
+                self.enumerate_recursive(&child_path, out)?;
+            }
+        }
+        Ok(())
+    }
+}
```

Add module registration in `crates/altium-format/src/lib.rs`:

```diff
--- a/crates/altium-format/src/lib.rs
+++ b/crates/altium-format/src/lib.rs
@@ -1,6 +1,7 @@
+mod cfb_document;
 pub mod document;
 pub mod intlib;
 pub mod pcbdoc;
 pub mod pcblib;
 pub mod project;
 pub mod schdoc;
 pub mod schlib;
```

---

### Milestone 3: Layer 2 — TrackedCfbDocument

**Files**: `crates/altium-format/src/tracked_cfb.rs`, `crates/altium-format/src/lib.rs`

**Requirements**:
- `TrackedCfbDocument` struct composing `CfbDocument` + `all_entries: HashSet<String>` + `consumed: HashSet<String>`
- `open(path)`: delegate to `CfbDocument::open`, then `enumerate_all_entries()` to populate `all_entries`
- `read_stream(path)`: mark as consumed, delegate to inner
- `read_stream_optional(path)`: mark as consumed (even if absent), delegate to inner
- `exists(path)`: delegate without marking consumed
- `list_entries(path)`: mark parent storage as consumed, delegate
- `assert_all_consumed()`: error with sorted list of unconsumed paths. Root storage `/` is always implicitly consumed.
- All types `pub(crate)`
- Register module in `lib.rs`

**Acceptance Criteria**:
- `cargo check -p altium-format` passes
- Opening BlankSchlibComponent.SchLib and reading all known streams + calling `assert_all_consumed()` passes
- Calling `assert_all_consumed()` without reading any streams returns `Err(UnconsumedStreams)` with the full entry list

**Tests**:
- **Test files**: `crates/altium-format/tests/tracked_cfb.rs`
- **Test type**: integration (real `.SchLib` files)
- **Backing**: user-specified
- **Scenarios**:
  - Normal: read all streams, assert_all_consumed succeeds
  - Error: assert_all_consumed fails with unconsumed list when streams not read
  - Edge: read_stream_optional marks absent stream as consumed (no false positive)

**Code Intent**:
- New file `tracked_cfb.rs`: `TrackedCfbDocument` struct
- `open`: `CfbDocument::open` → `enumerate_all_entries` → store both sets. Always insert `"/"` into consumed (root is implicit).
- `assert_all_consumed`: `all_entries.difference(&consumed)` → sort → if non-empty, `Err(UnconsumedStreams)`
- Add `mod tracked_cfb;` to `lib.rs` (private module)

### Code Changes

New file `crates/altium-format/src/tracked_cfb.rs`:

```diff
--- /dev/null
+++ b/crates/altium-format/src/tracked_cfb.rs
@@ -0,0 +1,58 @@
+//! Layer 2 of the 5-layer parsing stack: CFB stream consumption tracking.
+//! Wraps `CfbDocument` and records which entries have been read.
+//! `assert_all_consumed` enforces the invariant that every CFB stream is
+//! explicitly handled before `SchLib::open` returns.
+use std::collections::HashSet;
+use std::path::Path;
+
+use crate::cfb_document::CfbDocument;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) struct TrackedCfbDocument {
+    inner: CfbDocument,
+    all_entries: HashSet<String>,
+    consumed: HashSet<String>,
+}
+
+impl TrackedCfbDocument {
+    // Opens the CFB at `path`, enumerating all entries upfront for exhaustion tracking.
+    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self> {
+        let mut inner = CfbDocument::open(path)?;
+        let all_entries = inner.enumerate_all_entries()?;
+        let mut consumed = HashSet::new();
+        // Root storage is implicit; never appears as unconsumed.
+        consumed.insert("/".to_owned());
+        Ok(Self { inner, all_entries, consumed })
+    }
+
+    // Marks stream as consumed and reads it; returns StreamNotFound if absent.
+    pub(crate) fn read_stream(&mut self, path: &str) -> Result<Vec<u8>> {
+        self.consumed.insert(path.to_owned());
+        self.inner.read_stream(path)
+    }
+
+    // Marks stream as consumed (whether or not it exists) and reads it; returns Ok(None) if absent.
+    pub(crate) fn read_stream_optional(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
+        // Mark as consumed even when absent to avoid false-positive unconsumed errors.
+        self.consumed.insert(path.to_owned());
+        self.inner.read_stream_optional(path)
+    }
+
+    // Existence checks do not mark a stream as consumed; only read_stream and
+    // read_stream_optional claim ownership. Call read_stream_optional to both
+    // check and consume in one step.
+    pub(crate) fn exists(&self, path: &str) -> bool {
+        self.inner.exists(path)
+    }
+
+    // Marks the parent storage node as consumed and returns (storages, streams).
+    // Trailing slashes are stripped before insertion so "/Foo" and "/Foo/" are equivalent.
+    pub(crate) fn list_entries(&mut self, path: &str) -> Result<(Vec<String>, Vec<String>)> {
+        let normalized = path.trim_end_matches('/');
+        self.consumed.insert(normalized.to_owned());
+        self.inner.list_entries(normalized)
+    }
+
+    // Returns Err(UnconsumedStreams) if any enumerated entry was never read or listed.
+    // Call at the end of SchLib::open to enforce the total-consumption invariant.
+    pub(crate) fn assert_all_consumed(&self) -> Result<()> {
+        let mut unconsumed: Vec<String> = self
+            .all_entries
+            .difference(&self.consumed)
+            .cloned()
+            .collect();
+        if unconsumed.is_empty() {
+            return Ok(());
+        }
+        unconsumed.sort();
+        Err(AltiumFormatError::UnconsumedStreams { paths: unconsumed })
+    }
+}
```

Add module registration in `crates/altium-format/src/lib.rs`:

```diff
--- a/crates/altium-format/src/lib.rs
+++ b/crates/altium-format/src/lib.rs
@@ -1,5 +1,6 @@
 mod cfb_document;
+mod tracked_cfb;
 pub mod document;
 pub mod intlib;
 pub mod pcbdoc;
```

---

### Milestone 4: Layer 3 — Block Stream Parser

**Files**: `crates/altium-format/src/block_stream.rs`, `crates/altium-format/src/lib.rs`

**Flags**:
- `error-handling`: Invalid headers must produce `InvalidBlockHeader` with offset
- `needs-rationale`: Block header bit layout (size in 0-23, flags in 24-31)

**Requirements**:
- `BlockFormat` enum: `Text`, `Binary`
- `Block` struct: `format: BlockFormat`, `data: Vec<u8>`
- `parse_blocks(stream_data: &[u8]) -> Result<Vec<Block>>`: parse all blocks from raw bytes
  - Read i32 LE header, extract size (bits 0-23 via `& 0x00FF_FFFF`), flags (bits 24-31 via `>> 24`)
  - Flags 0x00 → Text, 0x01 → Binary, anything else → `InvalidBlockHeader`
  - Extract `size` bytes as payload
  - Validate entire stream is consumed (no trailing bytes)
- `iter_blocks(stream_data: &[u8]) -> BlockIter<'_>`: lazy iterator version
- All types `pub(crate)`
- Register module in `lib.rs`

**Acceptance Criteria**:
- `cargo check -p altium-format` passes
- Parsing `/FileHeader` stream from BlankSchlibComponent.SchLib produces at least 2 blocks (header + component)
- Block 0 has `format == Text`
- Trailing bytes after all blocks → `InvalidBlockHeader` or similar error

**Tests**:
- **Test files**: `crates/altium-format/tests/block_stream.rs`
- **Test type**: integration + property-based
- **Backing**: user-specified (integration), default-derived (property)
- **Scenarios**:
  - Normal: parse FileHeader from real SchLib, verify block count and formats
  - Property: roundtrip — construct valid block bytes, parse them, verify output matches
  - Edge: empty stream produces empty Vec
  - Error: truncated header (< 4 bytes) returns error
  - Error: payload extends past stream end returns error

**Code Intent**:
- New file `block_stream.rs`: `BlockFormat` enum, `Block` struct, `parse_blocks` function
- `parse_blocks`: cursor loop reading i32 LE, masking size/flags, slicing payload, advancing position
- `BlockIter`: struct holding `&[u8]` + position, implementing `Iterator<Item = Result<Block>>`
- Add `mod block_stream;` to `lib.rs`

### Code Changes

New file `crates/altium-format/src/block_stream.rs`:

```diff
--- /dev/null
+++ b/crates/altium-format/src/block_stream.rs
@@ -0,0 +1,80 @@
+//! Layer 3 of the 5-layer parsing stack: block-stream framing.
+//! Each Altium stream is a sequence of length-prefixed blocks.
+//! The 4-byte header encodes payload size (bits 0-23) and format (bits 24-31):
+//! 0x00 = text (pipe-delimited parameters), 0x01 = binary (packed struct).
+//! Unknown flag values are hard errors — Altium has no other documented formats.
+use crate::{AltiumFormatError, Result};
+
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub(crate) enum BlockFormat {
+    Text,
+    Binary,
+}
+
+#[derive(Debug, Clone)]
+pub(crate) struct Block {
+    pub(crate) format: BlockFormat,
+    pub(crate) data: Vec<u8>,
+}
+
+// Bits 0-23 carry the payload size; bits 24-31 carry the format discriminant.
+// 0x00 = text (pipe-delimited params), 0x01 = binary (packed struct).
+const SIZE_MASK: i32 = 0x00FF_FFFF;
+const FLAG_SHIFT: u32 = 24;
+
+// Parses all blocks from `stream_data` eagerly, returning an error on the first bad header.
+pub(crate) fn parse_blocks(stream_data: &[u8]) -> Result<Vec<Block>> {
+    let mut iter = BlockIter::new(stream_data);
+    let mut blocks = Vec::new();
+    for result in &mut iter {
+        blocks.push(result?);
+    }
+    Ok(blocks)
+}
+
+// Returns a lazy iterator over blocks; use when processing a stream incrementally.
+pub(crate) fn iter_blocks(stream_data: &[u8]) -> BlockIter<'_> {
+    BlockIter::new(stream_data)
+}
+
+pub(crate) struct BlockIter<'a> {
+    data: &'a [u8],
+    pos: usize,
+}
+
+impl<'a> BlockIter<'a> {
+    // Wraps a byte slice for lazy block parsing starting at position 0.
+    fn new(data: &'a [u8]) -> Self {
+        Self { data, pos: 0 }
+    }
+}
+
+impl<'a> Iterator for BlockIter<'a> {
+    type Item = Result<Block>;
+
+    fn next(&mut self) -> Option<Self::Item> {
+        if self.pos >= self.data.len() {
+            return None;
+        }
+        if self.data.len() - self.pos < 4 {
+            return Some(Err(AltiumFormatError::InvalidBlockHeader {
+                offset: self.pos,
+                detail: format!(
+                    "truncated header: only {} bytes remain",
+                    self.data.len() - self.pos
+                ),
+            }));
+        }
+        let header_offset = self.pos;
+        let header_bytes = [
+            self.data[self.pos],
+            self.data[self.pos + 1],
+            self.data[self.pos + 2],
+            self.data[self.pos + 3],
+        ];
+        let header = i32::from_le_bytes(header_bytes);
+        let size = (header & SIZE_MASK) as usize;
+        // Arithmetic right-shift of i32 by 24 leaves sign-extended bits 24-31 in the low byte.
+        // Casting to u8 truncates correctly regardless of sign extension.
+        let flags = (header >> FLAG_SHIFT) as u8;
+        self.pos += 4;
+        let format = match flags {
+            0x00 => BlockFormat::Text,
+            0x01 => BlockFormat::Binary,
+            other => {
+                return Some(Err(AltiumFormatError::InvalidBlockHeader {
+                    offset: header_offset,
+                    detail: format!("unknown flags byte {other:#04x}"),
+                }));
+            }
+        };
+        if self.pos + size > self.data.len() {
+            return Some(Err(AltiumFormatError::InvalidBlockHeader {
+                offset: header_offset,
+                detail: format!(
+                    "payload size {size} extends past stream end (stream has {} bytes, pos {})",
+                    self.data.len(),
+                    self.pos
+                ),
+            }));
+        }
+        let data = self.data[self.pos..self.pos + size].to_vec();
+        self.pos += size;
+        Some(Ok(Block { format, data }))
+    }
+}
```

Add module registration in `crates/altium-format/src/lib.rs`:

```diff
--- a/crates/altium-format/src/lib.rs
+++ b/crates/altium-format/src/lib.rs
@@ -1,6 +1,7 @@
+mod block_stream;
 mod cfb_document;
 mod tracked_cfb;
 pub mod document;
 pub mod intlib;
 pub mod pcbdoc;
```

---

### Milestone 5: Layer 4 — BinaryReader and BinaryWriter

**Files**: `crates/altium-format/src/binary_io.rs`, `crates/altium-format/src/lib.rs`

**Flags**:
- `error-handling`: Every read checks remaining bytes, returns `BinaryReadPastEnd`

**Requirements**:
- `BinaryReader<'a>`: cursor over `&[u8]` with position tracking
  - Primitive reads: `read_u8`, `read_i8`, `read_u16_le`, `read_i16_le`, `read_u32_le`, `read_i32_le`, `read_u64_le`, `read_i64_le`, `read_f32_le`, `read_f64_le`, `read_bool` (reads 1 byte: 0x00 = false, any non-zero = true — Decision: "Binary bool encoding"), `read_real48` (6-byte Borland Turbo Pascal Real → f64 — Decision: "BinaryReader provides read_real48")
  - Compound reads: `read_coord` (i32 LE as `Coord`), `read_coord_point` (two i32 LE as `CoordPoint`), `read_string_block` (i32 LE length + Windows-1252), `read_pascal_string` (u8 length + Windows-1252 bytes), `read_bytes(count)`, `skip(count)`, `sub_reader(len)`
  - Position/exhaustion: `remaining()`, `position()`, `assert_exhausted()`
  - Array helper: `read_array<T, N>(read_one)` for fixed-size arrays
- `BinaryWriter`: byte buffer writer
  - Mirror all BinaryReader methods as writes
  - `finish(self) -> Vec<u8>`
- All types `pub(crate)`
- Register module in `lib.rs`

**Acceptance Criteria**:
- `cargo check -p altium-format` passes
- Reading known byte sequences produces correct values
- `assert_exhausted` on fully-read reader succeeds
- `assert_exhausted` on partially-read reader returns `UnexpectedTrailingData`
- `sub_reader` creates independent reader that does not affect parent position beyond the reserved length

**Tests**:
- **Test files**: `crates/altium-format/tests/binary_io.rs`
- **Test type**: property-based + example-based
- **Backing**: default-derived
- **Scenarios**:
  - Property: write N primitives with BinaryWriter → read with BinaryReader → values match
  - Property: sub_reader(len) advances parent by exactly len bytes
  - Example: read_coord from `[0x10, 0x27, 0x00, 0x00]` = `Coord(10000)` (1 mil)
  - Edge: read_u8 from empty reader returns BinaryReadPastEnd
  - Edge: assert_exhausted after reading all bytes succeeds

**Code Intent**:
- New file `binary_io.rs`: `BinaryReader<'a>` struct with `data: &'a [u8]` and `pos: usize`
- Each `read_*` method: check `remaining() >= size`, slice bytes, convert via `from_le_bytes`, advance `pos`
- `sub_reader(len)`: check remaining, create new `BinaryReader` over `data[pos..pos+len]`, advance parent `pos` by `len`
- `BinaryWriter`: `buf: Vec<u8>`, each `write_*` extends via `extend_from_slice` with `to_le_bytes`
- Import `Coord` and `CoordPoint` from `altium-format-types`
- Add `mod binary_io;` to `lib.rs`

### Code Changes

New file `crates/altium-format/src/binary_io.rs`:

```diff
--- /dev/null
+++ b/crates/altium-format/src/binary_io.rs
@@ -0,0 +1,200 @@
+//! Layer 4 binary I/O for packed binary blocks.
+//! `BinaryReader` provides bounds-checked reads over a byte slice.
+//! `BinaryWriter` builds the corresponding byte sequence for serialization.
+//! All reads are little-endian. `assert_exhausted` enforces that every byte
+//! in a binary record is consumed before the dispatcher returns.
+use altium_format_types::{Coord, CoordPoint};
+
+use crate::{AltiumFormatError, Result};
+
+pub(crate) struct BinaryReader<'a> {
+    data: &'a [u8],
+    pos: usize,
+}
+
+impl<'a> BinaryReader<'a> {
+    // Creates a reader starting at byte 0 of `data`.
+    pub(crate) fn new(data: &'a [u8]) -> Self {
+        Self { data, pos: 0 }
+    }
+
+    // Returns the number of unread bytes remaining.
+    pub(crate) fn remaining(&self) -> usize {
+        self.data.len() - self.pos
+    }
+
+    // Returns the current byte offset (number of bytes already consumed).
+    pub(crate) fn position(&self) -> usize {
+        self.pos
+    }
+
+    // Returns `BinaryReadPastEnd` if fewer than `needed` bytes remain.
+    fn check_available(&self, needed: usize) -> Result<()> {
+        let available = self.remaining();
+        if available < needed {
+            Err(AltiumFormatError::BinaryReadPastEnd {
+                offset: self.pos,
+                needed,
+                available,
+            })
+        } else {
+            Ok(())
+        }
+    }
+
+    pub(crate) fn read_u8(&mut self) -> Result<u8> {
+        self.check_available(1)?;
+        let v = self.data[self.pos];
+        self.pos += 1;
+        Ok(v)
+    }
+
+    pub(crate) fn read_i8(&mut self) -> Result<i8> {
+        Ok(self.read_u8()? as i8)
+    }
+
+    pub(crate) fn read_u16_le(&mut self) -> Result<u16> {
+        self.check_available(2)?;
+        let v = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
+        self.pos += 2;
+        Ok(v)
+    }
+
+    pub(crate) fn read_i16_le(&mut self) -> Result<i16> {
+        self.check_available(2)?;
+        let v = i16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
+        self.pos += 2;
+        Ok(v)
+    }
+
+    pub(crate) fn read_u32_le(&mut self) -> Result<u32> {
+        self.check_available(4)?;
+        // `check_available` already verified `remaining() >= 4`, so the slice is exactly 4 bytes.
+        // `try_into().unwrap()` converts `&[u8]` to `[u8; 4]` and cannot fail here.
+        let v = u32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
+        self.pos += 4;
+        Ok(v)
+    }
+
+    pub(crate) fn read_i32_le(&mut self) -> Result<i32> {
+        self.check_available(4)?;
+        let v = i32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
+        self.pos += 4;
+        Ok(v)
+    }
+
+    pub(crate) fn read_u64_le(&mut self) -> Result<u64> {
+        self.check_available(8)?;
+        let v = u64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
+        self.pos += 8;
+        Ok(v)
+    }
+
+    pub(crate) fn read_i64_le(&mut self) -> Result<i64> {
+        self.check_available(8)?;
+        let v = i64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
+        self.pos += 8;
+        Ok(v)
+    }
+
+    pub(crate) fn read_f32_le(&mut self) -> Result<f32> {
+        self.check_available(4)?;
+        let v = f32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
+        self.pos += 4;
+        Ok(v)
+    }
+
+    pub(crate) fn read_f64_le(&mut self) -> Result<f64> {
+        self.check_available(8)?;
+        let v = f64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
+        self.pos += 8;
+        Ok(v)
+    }
+
+    // Reads a 6-byte Borland Turbo Pascal Real48 and converts to IEEE f64.
+    // Layout: byte[0] = 8-bit biased exponent, bytes[1..5] = 40-bit mantissa,
+    // MSB of byte[5] = sign bit. Exponent 0 means the value is 0.0.
+    // Algorithm verified against `Real48.cs` in decompiled Altium .NET source.
+    pub(crate) fn read_real48(&mut self) -> Result<f64> {
+        self.check_available(6)?;
+        let bytes = &self.data[self.pos..self.pos + 6];
+        self.pos += 6;
+        let exponent = bytes[0];
+        if exponent == 0 {
+            return Ok(0.0);
+        }
+        let sign = (bytes[5] & 0x80) as u64;
+        let mantissa = ((bytes[1] as u64)
+            | ((bytes[2] as u64) << 8)
+            | ((bytes[3] as u64) << 16)
+            | ((bytes[4] as u64) << 24)
+            | (((bytes[5] & 0x7F) as u64) << 32))
+            << 12;
+        let ieee_exp = (exponent as u64 - 129 + 1023) & 0x7FF;
+        let bits = (sign << 56) | (ieee_exp << 52) | (mantissa >> 1);
+        Ok(f64::from_bits(bits))
+    }
+
+    // 0x00 = false, any non-zero = true (Delphi Boolean convention).
+    pub(crate) fn read_bool(&mut self) -> Result<bool> {
+        Ok(self.read_u8()? != 0)
+    }
+
+    // Reads an i32 LE and wraps it as a Coord (DXP internal units).
+    pub(crate) fn read_coord(&mut self) -> Result<Coord> {
+        Ok(Coord::from_internal(self.read_i32_le()?))
+    }
+
+    // Reads two consecutive i32 LE values as (x, y) and returns a CoordPoint.
+    pub(crate) fn read_coord_point(&mut self) -> Result<CoordPoint> {
+        let x = self.read_coord()?;
+        let y = self.read_coord()?;
+        Ok(CoordPoint::new(x, y))
+    }
+
+    // Reads i32 LE length prefix then decodes that many bytes as Windows-1252.
+    pub(crate) fn read_string_block(&mut self) -> Result<String> {
+        let len = self.read_i32_le()? as usize;
+        self.check_available(len)?;
+        let bytes = &self.data[self.pos..self.pos + len];
+        // `decode_without_bom_handling`: skips BOM detection, which encoding_rs applies
+        // to the UTF-8 BOM (0xEF 0xBB 0xBF) when using `decode`. Altium binary strings
+        // carry no BOM; `decode_without_bom_handling` avoids any BOM-related byte consumption.
+        let (s, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(bytes);
+        self.pos += len;
+        Ok(s.into_owned())
+    }
+
+    // Reads u8 length prefix then decodes that many bytes as Windows-1252 (Pascal string format).
+    pub(crate) fn read_pascal_string(&mut self) -> Result<String> {
+        let len = self.read_u8()? as usize;
+        self.check_available(len)?;
+        let bytes = &self.data[self.pos..self.pos + len];
+        let (s, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(bytes);
+        self.pos += len;
+        Ok(s.into_owned())
+    }
+
+    // Returns a slice of the next `count` bytes without copying; advances position.
+    pub(crate) fn read_bytes(&mut self, count: usize) -> Result<&'a [u8]> {
+        self.check_available(count)?;
+        let slice = &self.data[self.pos..self.pos + count];
+        self.pos += count;
+        Ok(slice)
+    }
+
+    // Advances position by `count` bytes without reading them.
+    pub(crate) fn skip(&mut self, count: usize) -> Result<()> {
+        self.check_available(count)?;
+        self.pos += count;
+        Ok(())
+    }
+
+    // Creates a sub-reader over the next `len` bytes; advances parent by `len`.
+    pub(crate) fn sub_reader(&mut self, len: usize) -> Result<BinaryReader<'a>> {
+        self.check_available(len)?;
+        let sub = BinaryReader::new(&self.data[self.pos..self.pos + len]);
+        self.pos += len;
+        Ok(sub)
+    }
+
+    // Returns Err(UnexpectedTrailingData) if any unread bytes remain. Call after
+    // consuming all fields in a binary record to enforce the fail-fast invariant.
+    pub(crate) fn assert_exhausted(&self) -> Result<()> {
+        if self.remaining() == 0 {
+            Ok(())
+        } else {
+            Err(AltiumFormatError::UnexpectedTrailingData {
+                offset: self.pos,
+                count: self.remaining(),
+            })
+        }
+    }
+
+    // Calls `read_one` exactly N times and returns the results as a fixed-size array.
+    pub(crate) fn read_array<T, const N: usize>(
+        &mut self,
+        mut read_one: impl FnMut(&mut Self) -> Result<T>,
+    ) -> Result<[T; N]>
+    where
+        T: Copy + Default,
+    {
+        let mut arr = [T::default(); N];
+        for item in arr.iter_mut() {
+            *item = read_one(self)?;
+        }
+        Ok(arr)
+    }
+}
+
+pub(crate) struct BinaryWriter {
+    buf: Vec<u8>,
+}
+
+impl BinaryWriter {
+    // Creates an empty writer; grow by calling write_* methods.
+    pub(crate) fn new() -> Self {
+        Self { buf: Vec::new() }
+    }
+
+    pub(crate) fn write_u8(&mut self, v: u8) {
+        self.buf.push(v);
+    }
+
+    pub(crate) fn write_i8(&mut self, v: i8) {
+        self.buf.push(v as u8);
+    }
+
+    pub(crate) fn write_u16_le(&mut self, v: u16) {
+        self.buf.extend_from_slice(&v.to_le_bytes());
+    }
+
+    pub(crate) fn write_i16_le(&mut self, v: i16) {
+        self.buf.extend_from_slice(&v.to_le_bytes());
+    }
+
+    pub(crate) fn write_u32_le(&mut self, v: u32) {
+        self.buf.extend_from_slice(&v.to_le_bytes());
+    }
+
+    pub(crate) fn write_i32_le(&mut self, v: i32) {
+        self.buf.extend_from_slice(&v.to_le_bytes());
+    }
+
+    pub(crate) fn write_u64_le(&mut self, v: u64) {
+        self.buf.extend_from_slice(&v.to_le_bytes());
+    }
+
+    pub(crate) fn write_i64_le(&mut self, v: i64) {
+        self.buf.extend_from_slice(&v.to_le_bytes());
+    }
+
+    pub(crate) fn write_f32_le(&mut self, v: f32) {
+        self.buf.extend_from_slice(&v.to_le_bytes());
+    }
+
+    pub(crate) fn write_f64_le(&mut self, v: f64) {
+        self.buf.extend_from_slice(&v.to_le_bytes());
+    }
+
+    // 0x00 = false, 0x01 = true (Delphi Boolean convention; mirrors read_bool).
+    pub(crate) fn write_bool(&mut self, v: bool) {
+        self.buf.push(v as u8);
+    }
+
+    // Writes an IEEE f64 as a 6-byte Borland Turbo Pascal Real48. Inverse of read_real48.
+    pub(crate) fn write_real48(&mut self, v: f64) {
+        if v == 0.0 {
+            self.buf.extend_from_slice(&[0u8; 6]);
+            return;
+        }
+        let bits = v.to_bits();
+        let sign = ((bits >> 63) & 1) as u8;
+        let ieee_exp = ((bits >> 52) & 0x7FF) as i64;
+        let ieee_mant = bits & 0x000F_FFFF_FFFF_FFFF;
+        let real48_exp = (ieee_exp - 1023 + 129) as u8;
+        let shifted = ieee_mant << 1;
+        let b1 = (shifted >> 12) as u8;
+        let b2 = (shifted >> 20) as u8;
+        let b3 = (shifted >> 28) as u8;
+        let b4 = (shifted >> 36) as u8;
+        let b5 = ((shifted >> 44) as u8 & 0x7F) | (sign << 7);
+        self.buf.extend_from_slice(&[real48_exp, b1, b2, b3, b4, b5]);
+    }
+
+    // Writes a Coord as its i32 LE internal representation.
+    pub(crate) fn write_coord(&mut self, v: Coord) {
+        self.write_i32_le(v.to_internal());
+    }
+
+    // Writes a CoordPoint as two consecutive i32 LE values (x then y).
+    pub(crate) fn write_coord_point(&mut self, v: CoordPoint) {
+        self.write_coord(v.x);
+        self.write_coord(v.y);
+    }
+
+    // Appends raw bytes without any length prefix or encoding.
+    pub(crate) fn write_bytes(&mut self, bytes: &[u8]) {
+        self.buf.extend_from_slice(bytes);
+    }
+
+    // Writes i32 LE length prefix + Windows-1252 encoded string bytes.
+    pub(crate) fn write_string_block(&mut self, s: &str) {
+        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(s);
+        assert!(
+            encoded.len() <= i32::MAX as usize,
+            "string block too long: {} bytes (max {})", encoded.len(), i32::MAX
+        );
+        self.write_i32_le(encoded.len() as i32);
+        self.buf.extend_from_slice(&encoded);
+    }
+
+    // Writes u8 length prefix + Windows-1252 encoded string bytes (Pascal string).
+    pub(crate) fn write_pascal_string(&mut self, s: &str) {
+        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(s);
+        assert!(
+            encoded.len() <= 255,
+            "pascal string too long: {} bytes (max 255)", encoded.len()
+        );
+        self.write_u8(encoded.len() as u8);
+        self.buf.extend_from_slice(&encoded);
+    }
+
+    // Calls `write_one` for each element of `arr`; mirrors `BinaryReader::read_array`.
+    pub(crate) fn write_array<T, const N: usize>(
+        &mut self,
+        arr: &[T; N],
+        mut write_one: impl FnMut(&mut Self, &T),
+    ) {
+        for item in arr {
+            write_one(self, item);
+        }
+    }
+
+    // Consumes the writer and returns the assembled byte buffer.
+    pub(crate) fn finish(self) -> Vec<u8> {
+        self.buf
+    }
+}
```

Add module registration in `crates/altium-format/src/lib.rs`:

```diff
--- a/crates/altium-format/src/lib.rs
+++ b/crates/altium-format/src/lib.rs
@@ -1,6 +1,7 @@
+mod binary_io;
 mod block_stream;
 mod cfb_document;
 mod tracked_cfb;
 pub mod document;
 pub mod intlib;
```

---

### Milestone 6: Layer 4 — ParameterCollection

**Files**: `crates/altium-format/src/param_collection.rs`, `crates/altium-format/src/param_value.rs`, `crates/altium-format/src/lib.rs`

**Flags**:
- `needs-rationale`: IndexMap for insertion-order preservation; case-insensitive keys; escape sequences
- `error-handling`: Missing keys, parse failures, unknown remaining keys

**Requirements**:
- `ParameterCollection` struct with `params: IndexMap<String, String>` (keys stored in original case for round-trip serialization fidelity; lookups are case-insensitive via `.to_ascii_lowercase()` comparison; first occurrence wins for duplicates)
- `from_bytes(data: &[u8]) -> Result<Self>`: work from raw bytes to preserve encoding fidelity for `%UTF8%` keys: (1) strip trailing NUL byte; (2) split raw `&[u8]` on `0x7C` (`|`) — no decode yet; (3) filter empty segments; (4) for each segment, split on first `0x3D` (`=`) to get raw key bytes and raw value bytes; (5) decode key bytes as Windows-1252 to String; (6) if key starts with `%UTF8%`, strip prefix and decode value bytes as UTF-8 (return `Err` on invalid UTF-8); otherwise decode value bytes as Windows-1252; (7) unescape `[]` → `|`, `{}` → `=`, 0x8E → `|` (double 0x8E → literal 0x8E), and 0xA6 (broken bar) → `|` in the decoded value (Decision: "Unescape 0x8E and 0xA6 as pipe"); (8) insert original-case key (without `%UTF8%` prefix if stripped) and decoded value into IndexMap
- `from_str_params(s: &str) -> Result<Self>`: shared parse logic for already-decoded strings. Splits on `|`, filters empty, splits each segment on first `=`, unescapes value, inserts into IndexMap. Called by both `from_bytes` (after Windows-1252 decode) and `from_utf16le_bytes` (after UTF-16LE decode). Note: `%UTF8%` key handling only applies to `from_bytes` raw-byte path; `from_str_params` treats all values uniformly as already-decoded strings.
- `from_utf16le_bytes(data: &[u8]) -> Result<Self>`: UTF-16LE decode to `&str` → call `from_str_params` directly (no re-encode to bytes, no Windows-1252 re-decode)
- Consuming accessors:
  - `remove_required<T: FromParamValue>(key) -> Result<T>`: remove key (case-insensitive), parse value, error if missing
  - `remove_optional<T: FromParamValue>(key) -> Result<Option<T>>`: remove if present, parse, Ok(None) if absent
  - `remove_with_default<T: FromParamValue>(key, default) -> Result<T>`: remove if present, else default
  - `remove_coord(key, frac_key) -> Result<Coord>`: remove integer + frac pair, reconstruct `N * 100_000 + F`
  - `remove_indexed_coords(count_key, x_prefix, y_prefix) -> Result<Vec<CoordPoint>>`: indexed coordinate array
  - `remove_indexed<T>(count_key, base, parse_one) -> Result<Vec<T>>`: generic indexed family removal
  - `remove_list<T: FromParamValue>(key) -> Result<Vec<T>>`: comma-separated list
  - `remove_list_or_empty<T: FromParamValue>(key) -> Result<Vec<T>>`: same but Ok(vec![]) if absent
- Exhaustion: `remaining_keys()`, `remaining_count()`, `assert_exhausted()`
- `FromParamValue` trait: `fn from_param_value(value: &str) -> Result<Self>`
- `ToParamValue` trait: `fn to_param_value(&self) -> String`
- Implementations for: `i32`, `u32`, `i16`, `u16`, `i8`, `u8`, `f64`, `bool` (`T`/`F` and `TRUE`/`FALSE`), `String`, `Coord`, `Color`, `UniqueId`
- All types `pub(crate)` except traits (which may need `pub` for derive macros in future — but for now `pub(crate)`)
- Register modules in `lib.rs`

**Acceptance Criteria**:
- `cargo check -p altium-format` passes
- Parse `|RECORD=1|LIBREFERENCE=RES|LOCATION.X=100|LOCATION.X_FRAC=50000|\0` correctly
- Case-insensitive: key `record` matches `RECORD`
- Escape: `[]` in value decodes to `|`; `{}` decodes to `=`; 0x8E (142) in value decodes to `|`; double 0x8E decodes to literal 0x8E; 0xA6 (broken bar ¦) decodes to `|`
- `assert_exhausted` after consuming all keys succeeds
- `assert_exhausted` with remaining keys returns `UnknownParams` with key list
- `remove_coord("LOCATION.X", "LOCATION.X_FRAC")` returns `Coord(100 * 100_000 + 50_000)`
- `from_utf16le_bytes` parses valid UTF-16LE parameter string

**Tests**:
- **Test files**: `crates/altium-format/tests/param_collection.rs`
- **Test type**: property-based + example-based
- **Backing**: user-specified (example), default-derived (property)
- **Scenarios**:
  - Property: construct N key-value pairs, build pipe-delimited `|K1=V1|K2=V2|...\0` bytes manually, parse with from_bytes → keys/values match in insertion order, remove all via remove_required, assert_exhausted succeeds
  - Property: remove_required then assert_exhausted on single-key collection succeeds
  - Example: parse real FileHeader block 0 from BlankSchlibComponent.SchLib, extract HEADER and Weight
  - Example: escape sequences `[]` and `{}` decode correctly
  - Example: 0x8E in value decodes to `|`; double 0x8E decodes to literal 0x8E; 0xA6 (¦) decodes to `|`
  - Example: `%UTF8%` prefix on key name handled
  - Edge: empty parameter string `|\0` produces empty collection
  - Edge: duplicate keys — first occurrence wins
  - Error: remove_required on missing key returns MissingParam

**Code Intent**:
- New file `param_collection.rs`: `ParameterCollection` struct with `IndexMap<String, String>`
- `from_str_params(s: &str) -> Result<Self>`: shared parse logic for already-decoded strings — split on `|`, filter empty, split each segment on first `=` (if no `=` and segment non-empty return `Err(InvalidParamValue)`), unescape `[]`/`{}`/0x8E/0xA6 in value, insert into IndexMap
- `from_bytes`: strip trailing NUL from raw `&[u8]` → split raw bytes on `0x7C` (`|`) → filter empty → for each segment: split on first `0x3D` (`=`) to get raw key/value byte slices → decode key as Windows-1252 → if key has `%UTF8%` prefix: strip prefix, decode value as UTF-8 (Err on invalid); else: decode value as Windows-1252 → unescape `[]`/`{}`/0x8E/0xA6 in decoded value → insert into IndexMap. Splitting happens in raw bytes BEFORE decoding to preserve %UTF8% value integrity.
- `from_utf16le_bytes`: UTF-16LE decode to String → call `from_str_params` directly (avoids Windows-1252 re-decode which would corrupt non-ASCII)
- `remove_required`: iterate `params` keys, find case-insensitive match (compare `.to_ascii_lowercase()`), remove via `shift_remove` (preserves insertion order), parse value via `FromParamValue`
- `remove_coord`: remove integer key, remove optional frac key (default 0), compute `integer * 100_000 + frac`
- `remove_indexed`: read count from count_key, loop base..base+count calling closure
- New file `param_value.rs`: `FromParamValue` trait + `ToParamValue` trait + impls for primitives
- `bool` impl: match `"T"` | `"TRUE"` → true, `"F"` | `"FALSE"` → false, else error
- Add `mod param_collection; mod param_value;` to `lib.rs`

### Code Changes

New file `crates/altium-format/src/param_value.rs`:

```diff
--- /dev/null
+++ b/crates/altium-format/src/param_value.rs
@@ -0,0 +1,120 @@
+//! Conversion traits between raw Altium parameter strings and typed Rust values.
+//! `FromParamValue`: parse a string value for a named key into `T`.
+//! `ToParamValue`: serialize `T` back to the Altium string representation.
+//! `bool` uses Altium's T/F encoding, not Rust's true/false.
+use altium_format_types::Coord;
+
+use crate::{AltiumFormatError, Result};
+
+pub(crate) trait FromParamValue: Sized {
+    fn from_param_value(key: &str, value: &str) -> Result<Self>;
+}
+
+pub(crate) trait ToParamValue {
+    fn to_param_value(&self) -> String;
+}
+
+impl FromParamValue for String {
+    fn from_param_value(_key: &str, value: &str) -> Result<Self> {
+        Ok(value.to_owned())
+    }
+}
+
+impl ToParamValue for String {
+    fn to_param_value(&self) -> String {
+        self.clone()
+    }
+}
+
+impl FromParamValue for bool {
+    fn from_param_value(key: &str, value: &str) -> Result<Self> {
+        match value {
+            "T" | "TRUE" => Ok(true),
+            "F" | "FALSE" => Ok(false),
+            other => Err(AltiumFormatError::InvalidParamValue {
+                key: key.to_owned(),
+                detail: format!("expected T/F/TRUE/FALSE, got {other:?}"),
+            }),
+        }
+    }
+}
+
+impl ToParamValue for bool {
+    fn to_param_value(&self) -> String {
+        if *self { "T".to_owned() } else { "F".to_owned() }
+    }
+}
+
+macro_rules! impl_int_param_value {
+    ($($t:ty),+) => {
+        $(
+            impl FromParamValue for $t {
+                fn from_param_value(key: &str, value: &str) -> Result<Self> {
+                    value.parse::<$t>().map_err(|e| AltiumFormatError::InvalidParamValue {
+                        key: key.to_owned(),
+                        detail: e.to_string(),
+                    })
+                }
+            }
+
+            impl ToParamValue for $t {
+                fn to_param_value(&self) -> String {
+                    self.to_string()
+                }
+            }
+        )+
+    };
+}
+
+impl_int_param_value!(i8, u8, i16, u16, i32, u32, f64);
+
+impl FromParamValue for Coord {
+    fn from_param_value(key: &str, value: &str) -> Result<Self> {
+        let raw: i32 = value.parse().map_err(|e: std::num::ParseIntError| {
+            AltiumFormatError::InvalidParamValue { key: key.to_owned(), detail: e.to_string() }
+        })?;
+        Ok(Coord::from_internal(raw))
+    }
+}
+
+impl ToParamValue for Coord {
+    fn to_param_value(&self) -> String {
+        self.to_internal().to_string()
+    }
+}
+
+// usize is excluded from impl_int_param_value! because its width is platform-dependent
+// (32-bit or 64-bit depending on target); used for Weight and count fields.
+impl FromParamValue for usize {
+    fn from_param_value(key: &str, value: &str) -> Result<Self> {
+        value.parse::<usize>().map_err(|e| AltiumFormatError::InvalidParamValue {
+            key: key.to_owned(),
+            detail: e.to_string(),
+        })
+    }
+}
+
+impl ToParamValue for usize {
+    fn to_param_value(&self) -> String {
+        self.to_string()
+    }
+}
```

New file `crates/altium-format/src/param_collection.rs`:

```diff
--- /dev/null
+++ b/crates/altium-format/src/param_collection.rs
@@ -0,0 +1,221 @@
+//! Layer 4 parameter collection for text-format Altium blocks.
+//! Pipe-delimited key=value pairs decoded from Windows-1252 bytes.
+//! Keys stored in original case; lookups are case-insensitive.
+//! Accessors are destructive (remove-on-read): `assert_exhausted` then
+//! confirms every key was consumed, enforcing the fail-fast invariant.
+//! Insertion order is preserved (IndexMap) for deterministic serialization.
+use indexmap::IndexMap;
+use altium_format_types::{Coord, CoordPoint};
+
+use crate::param_value::{FromParamValue, ToParamValue};
+use crate::{AltiumFormatError, Result};
+
+pub(crate) struct ParameterCollection {
+    // Keys stored in original case for round-trip fidelity; lookups are case-insensitive.
+    // IndexMap preserves insertion order for deterministic serialization.
+    params: IndexMap<String, String>,
+}
+
+impl ParameterCollection {
+    // Creates an empty collection; use from_bytes or from_utf16le_bytes to populate.
+    pub(crate) fn new() -> Self {
+        Self { params: IndexMap::new() }
+    }
+
+    // Parses pipe-delimited Windows-1252 parameter bytes with %UTF8% key support.
+    // Splitting on raw bytes before decoding preserves %UTF8% value integrity.
+    pub(crate) fn from_bytes(data: &[u8]) -> Result<Self> {
+        let data = data.strip_suffix(b"\0").unwrap_or(data);
+        let mut params = IndexMap::new();
+        for segment in data.split(|&b| b == b'|') {
+            if segment.is_empty() {
+                continue;
+            }
+            let eq_pos = match segment.iter().position(|&b| b == b'=') {
+                Some(p) => p,
+                None => {
+                    let (key_str, _) =
+                        encoding_rs::WINDOWS_1252.decode_without_bom_handling(segment);
+                    return Err(AltiumFormatError::InvalidParamValue {
+                        key: key_str.into_owned(),
+                        detail: "segment has no '=' separator".to_owned(),
+                    });
+                }
+            };
+            let raw_key = &segment[..eq_pos];
+            let raw_value = &segment[eq_pos + 1..];
+            let (key_str, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(raw_key);
+            let key_str = key_str.into_owned();
+            let value_str = if key_str.starts_with("%UTF8%") {
+                let stripped_key = key_str[6..].to_owned();
+                let value = std::str::from_utf8(raw_value).map_err(|e| {
+                    AltiumFormatError::InvalidParamValue {
+                        key: stripped_key.clone(),
+                        detail: format!("UTF-8 decode error: {e}"),
+                    }
+                })?;
+                let unescaped = unescape_param_value(value);
+                params.entry(stripped_key).or_insert(unescaped);
+                continue;
+            } else {
+                let (decoded, _) =
+                    encoding_rs::WINDOWS_1252.decode_without_bom_handling(raw_value);
+                unescape_param_value(&decoded)
+            };
+            // First occurrence wins for duplicate keys.
+            params.entry(key_str).or_insert(value_str);
+        }
+        Ok(Self { params })
+    }
+
+    // Decodes UTF-16LE to &str then parses via from_str_params directly.
+    // Re-encoding to bytes then decoding via Windows-1252 would corrupt non-ASCII characters.
+    pub(crate) fn from_utf16le_bytes(data: &[u8]) -> Result<Self> {
+        let (decoded, _) = encoding_rs::UTF_16LE.decode_without_bom_handling(data);
+        Self::from_str_params(&decoded)
+    }
+
+    // Treats all values as already-decoded strings; %UTF8% key prefix handling
+    // does not apply here. Only from_bytes (raw-byte path) strips %UTF8% and
+    // switches to UTF-8 decoding for the value bytes.
+    fn from_str_params(s: &str) -> Result<Self> {
+        let s = s.strip_suffix('\0').unwrap_or(s);
+        let mut params = IndexMap::new();
+        for segment in s.split('|') {
+            if segment.is_empty() {
+                continue;
+            }
+            let eq_pos = match segment.find('=') {
+                Some(p) => p,
+                None => {
+                    return Err(AltiumFormatError::InvalidParamValue {
+                        key: segment.to_owned(),
+                        detail: "segment has no '=' separator".to_owned(),
+                    });
+                }
+            };
+            let key = &segment[..eq_pos];
+            let value = unescape_param_value(&segment[eq_pos + 1..]);
+            params.entry(key.to_owned()).or_insert(value);
+        }
+        Ok(Self { params })
+    }
+
+    // Removes key (case-insensitive), parses value via FromParamValue. Errors if absent.
+    // shift_remove preserves insertion order for the remaining keys (swap_remove would not).
+    pub(crate) fn remove_required<T: FromParamValue>(&mut self, key: &str) -> Result<T> {
+        let found = self.find_key(key).map(|k| k.to_owned());
+        match found {
+            Some(actual_key) => {
+                let value = self.params.shift_remove(&actual_key).unwrap();
+                T::from_param_value(&actual_key, &value)
+            }
+            None => Err(AltiumFormatError::MissingParam(key.to_owned())),
+        }
+    }
+
+    // Removes key (case-insensitive) and parses it if present; returns Ok(None) if absent.
+    pub(crate) fn remove_optional<T: FromParamValue>(
+        &mut self,
+        key: &str,
+    ) -> Result<Option<T>> {
+        let found = self.find_key(key).map(|k| k.to_owned());
+        match found {
+            Some(actual_key) => {
+                let value = self.params.shift_remove(&actual_key).unwrap();
+                T::from_param_value(&actual_key, &value).map(Some)
+            }
+            None => Ok(None),
+        }
+    }
+
+    // Removes and parses the key if present; returns `default` when the key is absent.
+    pub(crate) fn remove_with_default<T: FromParamValue>(
+        &mut self,
+        key: &str,
+        default: T,
+    ) -> Result<T> {
+        match self.remove_optional::<T>(key)? {
+            Some(v) => Ok(v),
+            None => Ok(default),
+        }
+    }
+
+    // Reconstructs a Coord from integer + fractional DXP parts: N * 100_000 + F.
+    pub(crate) fn remove_coord(&mut self, key: &str, frac_key: &str) -> Result<Coord> {
+        let integer: i32 = self.remove_required(key)?;
+        let frac: i32 = self.remove_with_default(frac_key, 0i32)?;
+        Ok(Coord::from_dxp_frac(integer, frac))
+    }
+
+    // Reads count from `count_key`, then removes `{x_prefix}N`/`{y_prefix}N` pairs as Coords.
+    pub(crate) fn remove_indexed_coords(
+        &mut self,
+        count_key: &str,
+        x_prefix: &str,
+        y_prefix: &str,
+    ) -> Result<Vec<CoordPoint>> {
+        let count: usize = self.remove_required(count_key)?;
+        let mut points = Vec::with_capacity(count);
+        for i in 0..count {
+            let x_key = format!("{x_prefix}{i}");
+            let y_key = format!("{y_prefix}{i}");
+            let x_frac_key = format!("{x_prefix}{i}_FRAC");
+            let y_frac_key = format!("{y_prefix}{i}_FRAC");
+            let x = self.remove_coord(&x_key, &x_frac_key)?;
+            let y = self.remove_coord(&y_key, &y_frac_key)?;
+            points.push(CoordPoint::new(x, y));
+        }
+        Ok(points)
+    }
+
+    // Reads count from `count_key`, then calls `parse_one(self, i)` for i in base..base+count.
+    pub(crate) fn remove_indexed<T>(
+        &mut self,
+        count_key: &str,
+        base: usize,
+        mut parse_one: impl FnMut(&mut Self, usize) -> Result<T>,
+    ) -> Result<Vec<T>> {
+        let count: usize = self.remove_required(count_key)?;
+        let mut items = Vec::with_capacity(count);
+        for i in base..base + count {
+            items.push(parse_one(self, i)?);
+        }
+        Ok(items)
+    }
+
+    // Removes a comma-separated value and parses each element; errors if key absent.
+    pub(crate) fn remove_list<T: FromParamValue>(&mut self, key: &str) -> Result<Vec<T>> {
+        let raw: String = self.remove_required(key)?;
+        raw.split(',')
+            .map(|s| T::from_param_value(key, s.trim()))
+            .collect()
+    }
+
+    // Like remove_list but returns empty Vec when the key is absent.
+    pub(crate) fn remove_list_or_empty<T: FromParamValue>(
+        &mut self,
+        key: &str,
+    ) -> Result<Vec<T>> {
+        match self.remove_optional::<String>(key)? {
+            Some(raw) => raw
+                .split(',')
+                .map(|s| T::from_param_value(key, s.trim()))
+                .collect(),
+            None => Ok(vec![]),
+        }
+    }
+
+    // Returns the keys that have not yet been consumed; used for debugging.
+    pub(crate) fn remaining_keys(&self) -> Vec<&str> {
+        self.params.keys().map(String::as_str).collect()
+    }
+
+    // Returns the count of unconsumed keys.
+    pub(crate) fn remaining_count(&self) -> usize {
+        self.params.len()
+    }
+
+    // Returns Err(UnknownParams) if any keys remain unconsumed.
+    // Call at the dispatch boundary after all known fields are removed.
+    pub(crate) fn assert_exhausted(&self) -> Result<()> {
+        if self.params.is_empty() {
+            return Ok(());
+        }
+        let keys: Vec<String> = self.params.keys().cloned().collect();
+        Err(AltiumFormatError::UnknownParams { keys })
+    }
+
+    // Returns the stored key whose lowercase form matches `key`, or `None` if absent.
+    fn find_key(&self, key: &str) -> Option<&str> {
+        let lower = key.to_ascii_lowercase();
+        self.params
+            .keys()
+            .find(|k| k.to_ascii_lowercase() == lower)
+            .map(String::as_str)
+    }
+}
+
+// Decodes Altium's in-value escape sequences.
+// Altium encodes literal pipe and equals inside values because | and = are delimiters.
+// Additionally, byte 0x8E (142) encodes a literal pipe within values (single 0x8E → |,
+// double 0x8E 0x8E → literal 0x8E character). Byte 0xA6 (broken bar ¦) is an alternate
+// pipe escape in ASCII format (¦ → |). Verified in `StrUtils.ReplaceSpecialDelimiterChars`
+// and `ProcessMBCSString` in decompiled .NET source.
+fn unescape_param_value(s: &str) -> String {
+    // Order matters: resolve double-0x8E first (literal 0x8E), then single 0x8E (pipe).
+    let s = s.replace("\u{008e}\u{008e}", "\x00");  // placeholder for literal 0x8E
+    let s = s.replace('\u{008e}', "|");
+    let s = s.replace('\x00', "\u{008e}");           // restore literal 0x8E
+    let s = s.replace('\u{00a6}', "|");               // broken bar → pipe
+    s.replace("[]", "|").replace("{}", "=")
+}
```

Add module registrations in `crates/altium-format/src/lib.rs`:

```diff
--- a/crates/altium-format/src/lib.rs
+++ b/crates/altium-format/src/lib.rs
@@ -1,6 +1,8 @@
 mod binary_io;
 mod block_stream;
 mod cfb_document;
+mod param_collection;
+mod param_value;
 mod tracked_cfb;
 pub mod document;
 pub mod intlib;
```

---

### Milestone 7: Layer 4 — Embedded Object Envelope Parser

**Files**: `crates/altium-format/src/embedded_object.rs`, `crates/altium-format/src/lib.rs`

**Flags**:
- `needs-rationale`: 0xD0 tag, inner header reuses block header bit layout
- `error-handling`: Invalid tag, truncated data, Weight mismatch

**Requirements**:
- `EmbeddedObject` struct: `id: String`, `inner_format: BlockFormat`, `inner_data: Vec<u8>`
- `parse_embedded_object(data: &[u8]) -> Result<EmbeddedObject>`: parse a single 0xD0 envelope from binary block payload
  - Read `0xD0` tag byte (error if wrong), `u8` id length, id string, i32 inner header (same bit layout as block header), inner data
- `parse_embedded_object_stream(blocks: &[Block]) -> Result<Vec<EmbeddedObject>>`:
  - Block 0: text, parse as ParameterCollection, consume RECORD + Weight, `assert_exhausted` internally
  - Blocks 1..N: binary, each parsed via `parse_embedded_object`
  - Validate entry count matches Weight
  - Returns only the parsed entries — header params are fully consumed inside (Decision: "parse_embedded_object_stream asserts exhaustion internally")
- All types `pub(crate)`
- Register module in `lib.rs`

**Acceptance Criteria**:
- `cargo check -p altium-format` passes
- Parse `/Storage` stream from BlankSchlibComponent.SchLib: header block has Weight, entry blocks parse correctly
- Weight mismatch returns `RecordCountMismatch`
- Invalid 0xD0 tag returns `InvalidEmbeddedObject`

**Tests**:
- **Test files**: `crates/altium-format/tests/embedded_object.rs`
- **Test type**: integration + example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: parse Storage stream from real SchLib, verify header params and entry count matches Weight
  - Example: hand-crafted 0xD0 envelope parses correctly
  - Error: wrong tag byte (0xE3 instead of 0xD0) returns error
  - Error: truncated data returns error

**Code Intent**:
- New file `embedded_object.rs`: `EmbeddedObject` struct, `parse_embedded_object`, `parse_embedded_object_stream`
- `parse_embedded_object`: `BinaryReader::new(data)` → `read_u8()` assert 0xD0 → `read_u8()` id_len → `read_bytes(id_len)` → String from bytes → `read_i32_le()` inner header → mask size/flags → `read_bytes(size)` → `reader.assert_exhausted()`
- `parse_embedded_object_stream`: validate `blocks[0].format == Text`, parse as `ParameterCollection`, `remove_optional::<i32>("RECORD")?` (header block carries RECORD=0 as a sentinel marker; consume it), `remove_required::<usize>("Weight")`, call `params.assert_exhausted()?` to enforce fail-fast on the header block, iterate `blocks[1..]` extracting `parse_embedded_object` from each binary block's data, assert count == Weight. Return `Result<Vec<EmbeddedObject>>` — header params are fully consumed inside, no live ParameterCollection returned.
- Add `mod embedded_object;` to `lib.rs`

### Code Changes

New file `crates/altium-format/src/embedded_object.rs`:

```diff
--- /dev/null
+++ b/crates/altium-format/src/embedded_object.rs
@@ -0,0 +1,70 @@
+//! Layer 4 parser for the embedded object envelope format.
+//! Each entry in a Storage or sidecar block stream is a 0xD0-tagged envelope:
+//! tag(1) + id_length(1) + id(N) + inner_header(4) + inner_data(M).
+//! The inner header uses the same bit layout as the block header (bits 0-23
+//! = size, bits 24-31 = format discriminant).
+//! `parse_embedded_object_stream` consumes the header block's params internally
+//! so callers never receive a partially-consumed `ParameterCollection`.
+use crate::binary_io::BinaryReader;
+use crate::block_stream::{Block, BlockFormat};
+use crate::param_collection::ParameterCollection;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) struct EmbeddedObject {
+    pub(crate) id: String,
+    pub(crate) inner_format: BlockFormat,
+    pub(crate) inner_data: Vec<u8>,
+}
+
+// Parses a single 0xD0-tagged embedded object envelope from a binary block payload.
+pub(crate) fn parse_embedded_object(data: &[u8]) -> Result<EmbeddedObject> {
+    let mut reader = BinaryReader::new(data);
+    let tag = reader.read_u8()?;
+    if tag != 0xD0 {
+        return Err(AltiumFormatError::InvalidEmbeddedObject(format!(
+            "expected 0xD0 tag, got {tag:#04x}"
+        )));
+    }
+    let id_len = reader.read_u8()? as usize;
+    let id_bytes = reader.read_bytes(id_len)?;
+    let id = String::from_utf8(id_bytes.to_vec()).map_err(|e| {
+        AltiumFormatError::InvalidEmbeddedObject(format!(
+            "embedded object id contains invalid UTF-8: {e}"
+        ))
+    })?;
+    let inner_header = reader.read_i32_le()?;
+    let inner_size = (inner_header & 0x00FF_FFFF) as usize;
+    let inner_flags = (inner_header >> 24) as u8;
+    let inner_format = match inner_flags {
+        0x00 => BlockFormat::Text,
+        0x01 => BlockFormat::Binary,
+        other => {
+            return Err(AltiumFormatError::InvalidEmbeddedObject(format!(
+                "unknown inner block flags {other:#04x}"
+            )));
+        }
+    };
+    let inner_data = reader.read_bytes(inner_size)?.to_vec();
+    reader.assert_exhausted()?;
+    Ok(EmbeddedObject { id, inner_format, inner_data })
+}
+
+// Parses the Storage-style block stream: block 0 = header params, blocks 1..N = entries.
+// Header params (RECORD, Weight) are consumed internally; callers receive only the entries.
+pub(crate) fn parse_embedded_object_stream(
+    blocks: &[Block],
+) -> Result<Vec<EmbeddedObject>> {
+    if blocks.is_empty() {
+        return Err(AltiumFormatError::InvalidEmbeddedObject(
+            "empty block list for embedded object stream".to_owned(),
+        ));
+    }
+    if blocks[0].format != BlockFormat::Text {
+        return Err(AltiumFormatError::InvalidEmbeddedObject(
+            "first block of embedded object stream must be text format".to_owned(),
+        ));
+    }
+    let mut params = ParameterCollection::from_bytes(&blocks[0].data)?;
+    // RECORD=0 sentinel may appear on the header block; consume it without dispatch.
+    params.remove_optional::<i32>("RECORD")?;
+    let weight: usize = params.remove_required("Weight")?;
+    params.assert_exhausted()?;
+    let entries: Result<Vec<EmbeddedObject>> =
+        blocks[1..].iter().map(|b| parse_embedded_object(&b.data)).collect();
+    let entries = entries?;
+    if entries.len() != weight {
+        return Err(AltiumFormatError::RecordCountMismatch {
+            section: "EmbeddedObjectStream".to_owned(),
+            expected: weight,
+            actual: entries.len(),
+        });
+    }
+    Ok(entries)
+}
```

Add module registration in `crates/altium-format/src/lib.rs`:

```diff
--- a/crates/altium-format/src/lib.rs
+++ b/crates/altium-format/src/lib.rs
@@ -1,7 +1,8 @@
 mod binary_io;
 mod block_stream;
 mod cfb_document;
+mod embedded_object;
 mod param_collection;
 mod param_value;
 mod tracked_cfb;
 pub mod document;
```

---

### Milestone 8: Layer 5 — Parsing Traits and SchRecord Enum Scaffold

**Files**: `crates/altium-format/src/sch/mod.rs`, `crates/altium-format/src/sch/records.rs`, `crates/altium-format/src/lib.rs`

**Flags**:
- `conformance`: Must match design doc dispatch pattern exactly
- `needs-rationale`: Exhaustion at dispatch boundary, not inside FromParams/FromBinary

**Requirements**:
- `FromParams` trait: `fn from_params(params: &mut ParameterCollection) -> Result<Self>`
- `ToParams` trait: `fn to_params(&self, params: &mut ParameterCollection)`
- `FromBinary` trait: `fn from_binary(reader: &mut BinaryReader<'_>) -> Result<Self>`
- `ToBinary` trait: `fn to_binary(&self, writer: &mut BinaryWriter)`
- `SchRecord` enum: initially scaffold with just the variants needed for a minimal SchLib (Component, Pin + a few primitives that appear in test files). All other RECORD values → `Err(UnknownRecordType)` to drive the red/green loop.
- `SchRecord::from_block(block: &Block) -> Result<Option<Self>>`:
  - Text: parse ParameterCollection, read RECORD, handle RECORD=0 sentinel → `Ok(None)`, RECORD=254 → read RECORDEX, dispatch, `assert_exhausted`
  - Binary: read code byte, dispatch (0x02 → Pin), `assert_exhausted`
- All types `pub(crate)` (traits + enum)
- Register module in `lib.rs`

**Acceptance Criteria**:
- `cargo check -p altium-format` passes
- `SchRecord::from_block` dispatches on RECORD value
- Unknown RECORD returns `UnknownRecordType`
- RECORD=0 returns `Ok(None)`
- Binary block with code 0x02 dispatches to Pin path

**Tests**:
- **Test files**: `crates/altium-format/tests/sch_records.rs`
- **Test type**: example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: text block with RECORD=1 dispatches to Component path (may fail on exhaustion until Component is implemented — that's expected in red/green)
  - Normal: RECORD=0 sentinel returns Ok(None)
  - Error: RECORD=999 returns UnknownRecordType(999)
  - Error: binary block with unknown code returns UnknownBinaryCode

**Code Intent**:
- New directory `crates/altium-format/src/sch/`
- New file `sch/mod.rs`: declare `pub(crate) mod records;`, re-export traits and SchRecord
- New file `sch/records.rs`: trait definitions (`FromParams`, `ToParams`, `FromBinary`, `ToBinary`), `SchRecord` enum, `SchRecord::from_block` dispatch function
- Initially stub record types as empty structs. Stub `FromParams` implementations return `Err(AltiumFormatError::UnknownRecordType(record_id))` and stub `FromBinary` implementations return `Err(AltiumFormatError::UnknownBinaryCode(code))` — the red/green loop surfaces clean errors (not panics) for unimplemented record types
- Add `mod sch;` to `lib.rs`

### Code Changes

New file `crates/altium-format/src/sch/mod.rs`:

```diff
--- /dev/null
+++ b/crates/altium-format/src/sch/mod.rs
@@ -0,0 +1,6 @@
+//! Schematic record types and parsing traits for the `sch` format family
+//! (SchLib, SchDoc). Layer 5 of the 5-layer parsing stack.
+pub(crate) mod records;
+
+pub(crate) use records::{
+    FromBinary, FromParams, SchRecord, ToBinary, ToParams,
+};
```

New file `crates/altium-format/src/sch/records.rs`:

```diff
--- /dev/null
+++ b/crates/altium-format/src/sch/records.rs
@@ -0,0 +1,70 @@
+//! Layer 5: parsing traits and schematic record types.
+//! `FromParams`/`FromBinary` implement record-specific field extraction.
+//! `assert_exhausted` is called at the dispatch boundary in `SchRecord::from_block`,
+//! not inside trait implementations, because base types and record-specific types
+//! share a single `ParameterCollection` (flatten pattern). An exhaustion check
+//! inside a base type would reject the record-specific fields that follow.
+use crate::binary_io::{BinaryReader, BinaryWriter};
+use crate::block_stream::{Block, BlockFormat};
+use crate::param_collection::ParameterCollection;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) trait FromParams: Sized {
+    fn from_params(params: &mut ParameterCollection) -> Result<Self>;
+}
+
+pub(crate) trait ToParams {
+    fn to_params(&self, params: &mut ParameterCollection);
+}
+
+pub(crate) trait FromBinary: Sized {
+    fn from_binary(reader: &mut BinaryReader<'_>) -> Result<Self>;
+}
+
+pub(crate) trait ToBinary {
+    fn to_binary(&self, writer: &mut BinaryWriter);
+}
+
+// Unknown RECORD values return Err(UnknownRecordType) (fail-fast: no silent skips).
+// SchComponent and SchPin stubs return Err to drive the red/green development loop;
+// callers receive a clean error rather than a panic for unimplemented record types.
+#[derive(Debug)]
+pub(crate) enum SchRecord {
+    Component(SchComponent),
+    Pin(SchPin),
+}
+
+// Each stub returns Err for its record type; the caller receives a structured error
+// rather than a panic, enabling the red/green loop to identify unimplemented types.
+#[derive(Debug, Default)]
+pub(crate) struct SchComponent;
+
+#[derive(Debug, Default)]
+pub(crate) struct SchPin;
+
+impl SchRecord {
+    // Dispatches a single block to the appropriate record constructor.
+    // Text blocks: parse ParameterCollection, read RECORD, dispatch, assert_exhausted.
+    // Binary blocks: read code byte, dispatch, assert_exhausted.
+    // RECORD=0 sentinel: returns Ok(None) — end of component data stream.
+    pub(crate) fn from_block(block: &Block) -> Result<Option<Self>> {
+        match block.format {
+            BlockFormat::Text => {
+                let mut params = ParameterCollection::from_bytes(&block.data)?;
+                let record_id: i32 = match params.remove_optional::<i32>("RECORD")? {
+                    Some(id) => id,
+                    None => {
+                        params.assert_exhausted()?;
+                        return Ok(None);
+                    }
+                };
+                if record_id == 0 {
+                    // assert_exhausted on sentinel: catches unknown params that may
+                    // appear alongside RECORD=0; silently dropping them would violate
+                    // the fail-fast invariant if Altium ever adds sentinel-block params.
+                    params.assert_exhausted()?;
+                    return Ok(None);
+                }
+                // RECORD=254 is Altium's extension mechanism: the real type is in RECORDEX.
+                // This allows record IDs >253 without breaking the 1-byte RECORD encoding.
+                let record_ex: Option<i32> = if record_id == 254 {
+                    Some(params.remove_required("RECORDEX")?)
+                } else {
+                    None
+                };
+                let effective_id = record_ex.unwrap_or(record_id);
+                let record = match effective_id {
+                    1 => SchRecord::Component(SchComponent::from_params(&mut params)?),
+                    other => return Err(AltiumFormatError::UnknownRecordType(other)),
+                };
+                params.assert_exhausted()?;
+                Ok(Some(record))
+            }
+            BlockFormat::Binary => {
+                let mut reader = BinaryReader::new(&block.data);
+                let code = reader.read_u8()?;
+                let record = match code {
+                    0x02 => SchRecord::Pin(SchPin::from_binary(&mut reader)?),
+                    other => return Err(AltiumFormatError::UnknownBinaryCode(other)),
+                };
+                reader.assert_exhausted()?;
+                Ok(Some(record))
+            }
+        }
+    }
+}
+
+impl FromParams for SchComponent {
+    // Returns UnknownRecordType(1) to surface unimplemented components via the red/green loop.
+    fn from_params(_params: &mut ParameterCollection) -> Result<Self> {
+        Err(AltiumFormatError::UnknownRecordType(1))
+    }
+}
+
+impl FromBinary for SchPin {
+    // Returns UnknownBinaryCode(0x02) to surface unimplemented binary pin parsing via the red/green loop.
+    fn from_binary(_reader: &mut BinaryReader<'_>) -> Result<Self> {
+        Err(AltiumFormatError::UnknownBinaryCode(0x02))
+    }
+}
```

Add module registration in `crates/altium-format/src/lib.rs`:

```diff
--- a/crates/altium-format/src/lib.rs
+++ b/crates/altium-format/src/lib.rs
@@ -1,8 +1,9 @@
 mod binary_io;
 mod block_stream;
 mod cfb_document;
 mod embedded_object;
 mod param_collection;
 mod param_value;
+mod sch;
 mod tracked_cfb;
 pub mod document;
```

---

### Milestone 9: SchLib Document Loader

**Files**: `crates/altium-format/src/schlib.rs`

**Flags**:
- `error-handling`: Stream tracking drives discovery — unimplemented streams produce clear errors
- `conformance`: Must follow design doc's SchLib stream manifest exactly
- `needs-rationale`: 9 pin sidecars in exact order; PinWideText overwrites PinDesc; /Storage decompression vs sidecar no-decompression

**Requirements**:
- `SchLib::open(path)`:
  - Open via `TrackedCfbDocument::open`
  - Read `/FileHeader` stream → `parse_blocks` → parse header block (consume RECORD if present, extract HEADER, Weight, MinorVersion, UniqueID, call `assert_exhausted`)
  - Read `/Storage` stream → `parse_blocks` → `parse_embedded_object_stream` → for each `EmbeddedObject`, decompress `inner_data` via `flate2::read::ZlibDecoder` (Storage entries are zlib-compressed; pin sidecar entries are NOT) → store decompressed bytes as embedded images
  - Read optional `/SectionKeys` → `parse_blocks` → `ParameterCollection::from_bytes` each block → `assert_exhausted` (drives red/green discovery)
  - Read optional `/LibAdditional` → `parse_blocks` → `ParameterCollection::from_bytes` each block → `assert_exhausted` (drives red/green discovery)
  - Enumerate root storages via `list_entries("/")` → identify component storages vs system storages
  - For each component storage: call `list_entries(&format!("/{key}"))` (no trailing slash) to mark the `/<key>` storage node as consumed AND discover its sub-streams. Then `read_stream("/<key>/Data")` → `parse_blocks` → dispatch via `SchRecord::from_block`
  - For alias components (storage has `Redirection` stream instead of `Data`): `read_stream("/{key}/Redirection")` → `parse_blocks` → `ParameterCollection::from_bytes` each block → `assert_exhausted` (drives red/green discovery of alias parameters)
  - Read optional per-component streams: `Additional` → parse as block-framed params
  - Parse pin sidecar streams for each component (see sidecar requirements below)
  - Call `assert_all_consumed()` at the end — no `skip_known` calls; every stream must be parsed or the system errors
- Implement `apply_pin_sidecar` helper function (per design doc pattern)
- Parse each sidecar in exact order:
  1. `PinFrac`: 12 bytes binary (3 × i32 LE: location_x_frac, location_y_frac, length_frac)
  2. `PinDesc`: i32 LE length + ASCII text
  3. `PinMiscData`: i32 LE length + UTF-16LE params
  4. `PinTextData`: 2-22 bytes variable binary
  5. `PinWideText`: i32 LE length + UTF-16LE params (overwrites PinDesc Name/Designator/Description)
  6. `PinSymbolLineWidth`: i32 LE length + UTF-16LE params
  7. `PinPackageLength`: i32 LE length + UTF-16LE params
  8. `PinPropagationDelay`: i32 LE length + UTF-16LE params
  9. `PinFunctionData`: i32 LE length + UTF-16LE params
- Each sidecar: `parse_blocks` → `parse_embedded_object_stream` → for each entry: parse `id` as pin index, apply inner data to pin. Sidecar streams are optional (absent = no entries), NOT compressed.
- Update `SchLib` struct to hold parsed data (components, records, embedded images)
- `SchLib` struct fields are `pub` (public API) but parsing internals remain `pub(crate)`

**Acceptance Criteria**:
- `cargo check -p altium-format` passes
- `altium validate data/BlankSchlibComponent.SchLib` exits successfully (may require implementing a few record types via the red/green loop)
- `altium validate data/LimeMicroAltiumLib_schLib.SchLib` passes (exercises pin sidecars)
- `assert_all_consumed` passes — every stream parsed, no `skip_known` used
- Unknown streams produce `UnconsumedStreams` error with paths
- Pin data is correctly merged from sidecar streams
- PinWideText values overwrite PinDesc values for the same fields

**Tests**:
- **Test files**: `crates/altium-format/tests/schlib.rs`
- **Test type**: integration (real files)
- **Backing**: user-specified
- **Scenarios**:
  - Normal: open BlankSchlibComponent.SchLib, verify component count
  - Normal: open LimeMicroAltiumLib_schLib.SchLib, verify multiple components discovered
  - Normal: parse SchLib with pins that have PinFrac data, verify coordinates include fractional parts
  - Normal: parse SchLib with PinWideText, verify Name/Designator overwritten
  - Edge: component with no sidecar streams (all optional, absent) — no errors
  - Error: file with unknown stream returns UnconsumedStreams

**Code Intent**:
- Rewrite `schlib.rs`: replace stub with full loader using TrackedCfbDocument
- `SchLib` struct: `components: Vec<SchLibComponent>` where `SchLibComponent` holds component name, records, pin data
- `open`: TrackedCfbDocument::open → read FileHeader → read Storage (decompress via flate2 ZlibDecoder) → read SectionKeys (parse blocks + assert_exhausted each block) → read LibAdditional (parse blocks + assert_exhausted each block) → enumerate components → for each: list_entries("/<key>") (no trailing slash) to consume storage node + read Data + parse blocks + dispatch records + parse all sidecar streams → handle aliases (read Redirection stream, parse blocks, assert_exhausted each block) → assert_all_consumed
- FileHeader parsing: parse_blocks, block 0 is header params. Call `params.remove_optional::<i32>("RECORD")?` first (SchLib FileHeader block may carry RECORD=0; consume it). Then extract HEADER, Weight (if present), MinorVersion, UniqueID via `remove_required`/`remove_optional`. Call `params.assert_exhausted()?` after all known fields are consumed — future Altium versions may add new FileHeader params, and silently dropping them violates the fail-fast policy.
- Component discovery: list_entries("/"), filter out "Storage"/"FileHeader"/"SectionKeys"/"LibAdditional". For each remaining storage: call list_entries("/<key>") (no trailing slash) — this marks the storage node as consumed and returns its sub-streams. Check for "Data" sub-stream (canonical component) or "Redirection" sub-stream (alias component). Storages with neither are unknown — return an error.
- Pin sidecar loading: add `apply_pin_sidecar` function (per design doc pattern). After dispatching records from Data stream, collect pins into a mutable Vec. Call `apply_pin_sidecar` for each of the 9 sidecar stream names in exact order. In the `apply_pin_sidecar` body (NOT in the closure — the closure receives `&mut SchPin`, not an index): parse `entry.id` as `pin_index: usize`, bounds-check `pin_index < pins.len()` — out-of-bounds returns `Err(InvalidEmbeddedObject(format!("pin index {pin_index} out of bounds (...)")))` (Decision: "Pin sidecar index out-of-bounds returns Err") — then call `apply(entry, &mut pins[pin_index])?`. Each sidecar's apply closure: construct `BinaryReader` or `ParameterCollection` from inner data (NOT decompressed — sidecars are uncompressed unlike /Storage), extract fields, merge into the pin, call `assert_exhausted` on reader/params.
- Unknown component storages (neither Data nor Redirection): return `Err(CfbError(format!("storage /{key}/ has neither Data nor Redirection stream")))` (Decision: "Unknown component storage returns CfbError")

### Code Changes

Rewrite `crates/altium-format/src/schlib.rs`:

```diff
--- a/crates/altium-format/src/schlib.rs
+++ b/crates/altium-format/src/schlib.rs
@@ -1,13 +1,174 @@
+//! SchLib document loader: top-level entry point for the 5-layer parsing stack.
+//! Opens a `.SchLib` CFB container, reads all required and optional streams,
+//! dispatches records via `SchRecord::from_block`, and merges pin sidecar data.
+//! The loader calls `assert_all_consumed` at exit — every CFB stream must be
+//! explicitly handled. Unknown streams fail the call rather than being silently skipped.
 use std::path::Path;

-pub struct SchLib {
-    // TODO: Define the structure
+use flate2::read::ZlibDecoder;
+use std::io::Read;
+
+use crate::block_stream::parse_blocks;
+use crate::embedded_object::{parse_embedded_object_stream, EmbeddedObject};
+use crate::param_collection::ParameterCollection;
+use crate::sch::records::{SchPin, SchRecord};
+use crate::tracked_cfb::TrackedCfbDocument;
+use crate::{AltiumFormatError, Result};
+
+pub struct SchLib {
+    pub components: Vec<SchLibComponent>,
+    pub embedded_images: Vec<Vec<u8>>,
 }

-impl SchLib {
-    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
-        let path = path.as_ref();
-        let _file = std::fs::File::open(path)?;
-        Ok(Self {})
-    }
+pub struct SchLibComponent {
+    pub name: String,
+    pub records: Vec<SchRecord>,
+    pub pins: Vec<SchPin>,
 }
+
+impl SchLib {
+    // Opens a `.SchLib` file, parses all streams, and returns the populated document.
+    // Every CFB stream must be consumed; unknown streams produce `UnconsumedStreams`.
+    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
+        let mut cfb = TrackedCfbDocument::open(path)?;
+
+        parse_file_header(&mut cfb)?;
+
+        // Storage: block stream of zlib-compressed embedded images.
+        let storage_bytes = cfb.read_stream("/Storage")?;
+        let storage_blocks = parse_blocks(&storage_bytes)?;
+        let raw_objects = parse_embedded_object_stream(&storage_blocks)?;
+        let mut embedded_images = Vec::new();
+        for obj in &raw_objects {
+            let mut decoder = ZlibDecoder::new(obj.inner_data.as_slice());
+            let mut decompressed = Vec::new();
+            decoder.read_to_end(&mut decompressed).map_err(|e| {
+                AltiumFormatError::DecompressionError(e.to_string())
+            })?;
+            embedded_images.push(decompressed);
+        }
+
+        if let Some(sk_bytes) = cfb.read_stream_optional("/SectionKeys")? {
+            let sk_blocks = parse_blocks(&sk_bytes)?;
+            for block in &sk_blocks {
+                let mut params = ParameterCollection::from_bytes(&block.data)?;
+                params.assert_exhausted()?;
+            }
+        }
+
+        if let Some(la_bytes) = cfb.read_stream_optional("/LibAdditional")? {
+            let la_blocks = parse_blocks(&la_bytes)?;
+            for block in &la_blocks {
+                let mut params = ParameterCollection::from_bytes(&block.data)?;
+                params.assert_exhausted()?;
+            }
+        }
+
+        // Enumerate root-level entries to discover component storages.
+        let (root_storages, _root_streams) = cfb.list_entries("/")?;
+        let system_keys = ["Storage", "FileHeader", "SectionKeys", "LibAdditional"];
+        let component_keys: Vec<String> = root_storages
+            .into_iter()
+            .filter(|k| !system_keys.contains(&k.as_str()))
+            .collect();
+
+        let mut components = Vec::new();
+        for key in &component_keys {
+            if let Some(component) = parse_component(&mut cfb, &key, &embedded_images)? {
+                components.push(component);
+            }
+        }
+
+        cfb.assert_all_consumed()?;
+        Ok(Self { components, embedded_images })
+    }
+}
+
+// Reads and exhaustion-checks the /FileHeader stream. All fields are optional
+// because real SchLib files vary in which FileHeader params are present.
+fn parse_file_header(cfb: &mut TrackedCfbDocument) -> Result<()> {
+    let fh_bytes = cfb.read_stream("/FileHeader")?;
+    let fh_blocks = parse_blocks(&fh_bytes)?;
+    if fh_blocks.len() > 1 {
+        return Err(AltiumFormatError::RecordCountMismatch {
+            section: "FileHeader".to_owned(),
+            expected: 1,
+            actual: fh_blocks.len(),
+        });
+    }
+    if !fh_blocks.is_empty() {
+        let mut params = ParameterCollection::from_bytes(&fh_blocks[0].data)?;
+        // SchLib FileHeader block may carry RECORD=0 (same sentinel as Data streams).
+        params.remove_optional::<i32>("RECORD")?;
+        params.remove_optional::<String>("HEADER")?;
+        params.remove_optional::<i32>("Weight")?;
+        params.remove_optional::<String>("MinorVersion")?;
+        params.remove_optional::<String>("UniqueID")?;
+        params.assert_exhausted()?;
+    }
+    Ok(())
+}
+
+// Parses one component storage under `/{key}`. Returns `None` for alias components
+// (Redirection stream present) so the caller's component Vec excludes aliases.
+fn parse_component(
+    cfb: &mut TrackedCfbDocument,
+    key: &str,
+    _storage_images: &[Vec<u8>],
+) -> Result<Option<SchLibComponent>> {
+    let storage_path = format!("/{key}");
+    let (_, sub_streams) = cfb.list_entries(&storage_path)?;
+
+    if sub_streams.iter().any(|s| s == "Redirection") {
+        // Alias component: Redirection stream replaces Data; parse and assert_exhausted.
+        let redir_path = format!("/{key}/Redirection");
+        let redir_bytes = cfb.read_stream(&redir_path)?;
+        let redir_blocks = parse_blocks(&redir_bytes)?;
+        for block in &redir_blocks {
+            let mut params = ParameterCollection::from_bytes(&block.data)?;
+            params.assert_exhausted()?;
+        }
+        return Ok(None);
+    }
+
+    if !sub_streams.iter().any(|s| s == "Data") {
+        // Neither Data nor Redirection: malformed component storage; Altium requires one of the two.
+        return Err(AltiumFormatError::CfbError(format!(
+            "storage /{key}/ has neither Data nor Redirection stream"
+        )));
+    }
+
+    // Optional per-component Additional stream: parse as block-framed params.
+    let additional_path = format!("/{key}/Additional");
+    if let Some(additional_bytes) = cfb.read_stream_optional(&additional_path)? {
+        let additional_blocks = parse_blocks(&additional_bytes)?;
+        for block in &additional_blocks {
+            let mut params = ParameterCollection::from_bytes(&block.data)?;
+            params.assert_exhausted()?;
+        }
+    }
+
+    let data_path = format!("/{key}/Data");
+    let data_bytes = cfb.read_stream(&data_path)?;
+    let data_blocks = parse_blocks(&data_bytes)?;
+
+    let mut records = Vec::new();
+    let mut pins = Vec::new();
+    let mut sentinel_idx = None;
+    for (i, block) in data_blocks.iter().enumerate() {
+        match SchRecord::from_block(block)? {
+            Some(SchRecord::Pin(pin)) => pins.push(pin),
+            Some(record) => records.push(record),
+            None => { sentinel_idx = Some(i); break; }
+        }
+    }
+    if let Some(idx) = sentinel_idx {
+        if idx + 1 < data_blocks.len() {
+            return Err(AltiumFormatError::InvalidParamValue {
+                key: "Data".to_owned(),
+                detail: format!(
+                    "found {} blocks after RECORD=0 sentinel",
+                    data_blocks.len() - idx - 1
+                ),
+            });
+        }
+    }
+
+    // Apply pin sidecars in exact order; PinWideText overwrites PinDesc fields.
+    // Sidecar inner_data is NOT zlib-compressed (unlike /Storage entries).
+    let sidecar_names = [
+        "PinFrac",
+        "PinDesc",
+        "PinMiscData",
+        "PinTextData",
+        "PinWideText",
+        "PinSymbolLineWidth",
+        "PinPackageLength",
+        "PinPropagationDelay",
+        "PinFunctionData",
+    ];
+    for sidecar_name in &sidecar_names {
+        let sidecar_path = format!("/{key}/{sidecar_name}");
+        if let Some(sidecar_bytes) = cfb.read_stream_optional(&sidecar_path)? {
+            let sidecar_blocks = parse_blocks(&sidecar_bytes)?;
+            let entries = parse_embedded_object_stream(&sidecar_blocks)?;
+            apply_pin_sidecar(&entries, &mut pins, sidecar_name)?;
+        }
+    }
+
+    Ok(Some(SchLibComponent { name: key.to_owned(), records, pins }))
+}
+
+// Processes sidecar entries by index into the pin list.
+// Out-of-bounds pin index is a malformed-file condition, not a programming error.
+// Each sidecar type requires a dedicated decode branch; unknown types return an error.
+fn apply_pin_sidecar(
+    entries: &[EmbeddedObject],
+    pins: &mut Vec<SchPin>,
+    sidecar_name: &str,
+) -> Result<()> {
+    for entry in entries {
+        let pin_index: usize = entry.id.parse().map_err(|_| {
+            AltiumFormatError::InvalidEmbeddedObject(format!(
+                "pin sidecar id '{}' is not a valid pin index", entry.id
+            ))
+        })?;
+        if pin_index >= pins.len() {
+            return Err(AltiumFormatError::InvalidEmbeddedObject(format!(
+                "pin sidecar index {} out of bounds (have {} pins)", pin_index, pins.len()
+            )));
+        }
+        return Err(AltiumFormatError::InvalidEmbeddedObject(format!(
+            "unsupported sidecar type '{}' for pin {}",
+            sidecar_name, pin_index
+        )));
+    }
+    Ok(())
+}
```

---

### Milestone 10: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `crates/altium-format/src/CLAUDE.md` (index update for new modules)
- `crates/altium-format/src/sch/CLAUDE.md` (index for sch/ directory)
- `crates/altium-format/src/README.md` (invisible knowledge: 5-layer architecture, exhaustion invariants, data flow)

**Requirements**:

Delegate to Technical Writer. Documentation format per `~/.claude/conventions/documentation.md`.

Key deliverables:
- CLAUDE.md: Pure navigation index (tabular format) for new modules
- README.md: Invisible knowledge about 5-layer architecture, exhaustion boundaries, pin sidecar ordering, ParameterCollection insertion-order invariant

**Acceptance Criteria**:
- CLAUDE.md is tabular index only (no prose sections)
- README.md exists with invisible knowledge
- README.md is self-contained (no external references)
- Architecture diagram in README.md matches plan's Invisible Knowledge section

### Code Changes

Skip reason: documentation-only milestone. Delegated to @agent-technical-writer.

## Milestone Dependencies

```
M1 (Error Types)
  │
  ├──► M2 (CfbDocument)
  │      │
  │      └──► M3 (TrackedCfbDocument)
  │             │
  │             └──────────────────────────────────► M9 (SchLib Loader)
  │                                                    │
  ├──► M4 (Block Stream Parser) ──────────────────────►│
  │                                                    │
  ├──► M5 (BinaryReader/Writer) ──► M7 (Embedded Obj) ►│
  │                                                    │
  ├──► M6 (ParameterCollection) ──► M7 ───────────────►│
  │                                     │              │
  │                                     └──► M8 ──────►│
  │                                    (SchRecord)     │
  │                                                    │
  └───────────────────────────────────────────────────► M10 (Docs)
```

**Parallel waves**:
- Wave 1: M1 (Error Types) — blocks everything
- Wave 2: M2, M4, M5, M6 — independent Layer 1/3/4 modules (can be parallel)
- Wave 3: M3, M7, M8 — depend on Wave 2 outputs
- Wave 4: M9 — depends on M3, M4, M6, M7, M8 (includes sidecars — no skip_known)
- Wave 5: M10 — depends on all
