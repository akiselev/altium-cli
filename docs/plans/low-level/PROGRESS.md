# Low-Level API Implementation Progress

## Milestone 1: Error Types Expansion
- **Status**: COMPLETE
- **Date**: 2026-02-22
- **Files modified**: `crates/altium-format/src/lib.rs`
- **Verification**: `cargo check --workspace` passes
- **Summary**: Expanded `AltiumFormatError` from 4 variants to 15 variants covering all 5 layers of the parsing stack. Replaced `InvalidParamValue(String)` with structured `InvalidParamValue { key, detail }` and `BinaryParsingError(String)` with `BinaryReadPastEnd { offset, needed, available }`. Added CFB, stream tracking, block framing, parameter collection, embedded object, and record dispatch error variants.

## Milestone 2: Layer 1 — CfbDocument
- **Status**: COMPLETE
- **Date**: 2026-02-22
- **Files created**: `crates/altium-format/src/cfb_document.rs`
- **Files modified**: `crates/altium-format/src/lib.rs`
- **Verification**: `cargo check -p altium-format` passes, `cargo test -p altium-format -- cfb_document` — 5/5 tests pass
- **Summary**: Implemented `CfbDocument` struct wrapping `cfb::CompoundFile<Cursor<Vec<u8>>>` with `open`, `read_stream`, `read_stream_optional`, `exists`, `list_entries`, and `enumerate_all_entries` methods. All types `pub(crate)`. Module registered as private `mod cfb_document` in `lib.rs`. Five unit tests cover: open + read FileHeader, enumerate entries with specific path assertions, optional missing stream returns None, non-existent file returns Io error, missing stream returns StreamNotFound.

## Milestone 3: Layer 2 — TrackedCfbDocument
- **Status**: COMPLETE
- **Date**: 2026-02-22
- **Files created**: `crates/altium-format/src/tracked_cfb.rs`
- **Files modified**: `crates/altium-format/src/lib.rs`
- **Verification**: `cargo check -p altium-format` passes, `cargo test -p altium-format` — 8/8 tests pass (5 cfb_document + 3 tracked_cfb)
- **Summary**: Implemented `TrackedCfbDocument` struct composing `CfbDocument` + `all_entries: HashSet<String>` + `consumed: HashSet<String>`. Methods: `open` (delegates to CfbDocument, enumerates all entries, pre-seeds root `/` as consumed), `read_stream` (marks consumed, delegates), `read_stream_optional` (marks consumed even if absent, delegates), `exists` (delegates without marking), `list_entries` (normalizes path with root `/` guard, marks storage as consumed, delegates), `assert_all_consumed` (set difference → sorted unconsumed list → `Err(UnconsumedStreams)`). All types `pub(crate)`. Three inline unit tests: full consumption succeeds, no-read fails with all 4 known paths, optional absent stream causes no false positive.
