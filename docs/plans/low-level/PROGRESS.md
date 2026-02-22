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

## Milestone 4: Layer 3 — Block Stream Parser
- **Status**: COMPLETE
- **Date**: 2026-02-22
- **Files created**: `crates/altium-format/src/block_stream.rs`
- **Files modified**: `crates/altium-format/src/lib.rs`
- **Verification**: `cargo check -p altium-format` passes, `cargo test -p altium-format -- block_stream` — 8/8 tests pass
- **Summary**: Implemented `BlockFormat` enum (`Text`, `Binary`), `Block` struct, `parse_blocks` eager parser, `iter_blocks` lazy iterator via `BlockIter<'a>`. Block header uses i32 LE with bits 0-23 as payload size (`& 0x00FF_FFFF`) and bits 24-31 as format flags (`>> 24`). Flags 0x00 = Text, 0x01 = Binary, anything else → `InvalidBlockHeader`. Eight inline unit tests: empty stream, single text/binary, multiple blocks, truncated header, payload past end, unknown flags, iterator matches eager.

## Milestone 5: Layer 4 — BinaryReader and BinaryWriter
- **Status**: COMPLETE
- **Date**: 2026-02-22
- **Files created**: `crates/altium-format/src/binary_io.rs`
- **Files modified**: `crates/altium-format/src/lib.rs`
- **Verification**: `cargo check -p altium-format` passes, `cargo test -p altium-format -- binary_io` — 18/18 tests pass
- **Summary**: Implemented `BinaryReader<'a>` with bounds-checked reads for all primitive types (u8/i8/u16/i16/u32/i32/u64/i64/f32/f64), `read_real48` (6-byte Borland Turbo Pascal Real → f64), `read_bool` (Delphi convention), `read_coord`/`read_coord_point`, `read_string_block`/`read_pascal_string` (Windows-1252 via `encoding_rs`), `read_bytes`, `skip`, `sub_reader`, `assert_exhausted`, and `read_array<T, N>`. Implemented `BinaryWriter` with all mirror methods plus `write_real48` (inverse conversion). Eighteen inline unit tests cover roundtrips for all types, sub-reader parent advancement, exhaustion checks, and edge cases.

## Milestone 6: Layer 4 — ParameterCollection
- **Status**: COMPLETE
- **Date**: 2026-02-22
- **Files created**: `crates/altium-format/src/param_value.rs`, `crates/altium-format/src/param_collection.rs`
- **Files modified**: `crates/altium-format/src/lib.rs`
- **Verification**: `cargo check -p altium-format` passes, `cargo test -p altium-format -- param_collection` — 25/25 tests pass
- **Summary**: Implemented `FromParamValue`/`ToParamValue` traits with impls for `String`, `bool` (T/F/TRUE/FALSE), `i8`/`u8`/`i16`/`u16`/`i32`/`u32`/`f64`/`usize`, and `Coord`. Implemented `ParameterCollection` with `IndexMap<String, String>` for insertion-order preservation: `from_bytes` (Windows-1252 with `%UTF8%` key support, raw-byte splitting before decode), `from_utf16le_bytes`, case-insensitive remove-on-read accessors (`remove_required`, `remove_optional`, `remove_with_default`, `remove_coord`, `remove_indexed_coords`, `remove_indexed`, `remove_list`, `remove_list_or_empty`), and `assert_exhausted`. Value unescape handles `[]` → `|`, `{}` → `=`, 0x8E (U+017D Ž) → `|`, double 0x8E → literal Ž, 0xA6 (¦) → `|`. Twenty-five inline unit tests cover parsing, case insensitivity, escape sequences, UTF-8 keys, DXP coord reconstruction, and exhaustion.

## Milestone 7: Layer 4 — Embedded Object Envelope Parser
- **Status**: COMPLETE
- **Date**: 2026-02-22
- **Files created**: `crates/altium-format/src/embedded_object.rs`
- **Files modified**: `crates/altium-format/src/lib.rs`
- **Verification**: `cargo check -p altium-format` passes, `cargo test -p altium-format -- embedded_object` — 7/7 tests pass
- **Summary**: Implemented `EmbeddedObject` struct (`id`, `inner_format`, `inner_data`), `parse_embedded_object` (0xD0 tag, u8 id length, id string, i32 inner header with same block header bit layout, inner data, `assert_exhausted`), and `parse_embedded_object_stream` (block 0 text header with RECORD + Weight consumed internally + `assert_exhausted`, blocks 1..N as binary envelopes, Weight validation via `RecordCountMismatch`). Seven inline unit tests: text/binary envelopes, wrong tag, truncated data, stream with weight, weight mismatch, empty blocks.

## Milestone 8: Layer 5 — Parsing Traits and SchRecord Enum Scaffold
- **Status**: COMPLETE
- **Date**: 2026-02-22
- **Files created**: `crates/altium-format/src/sch/mod.rs`, `crates/altium-format/src/sch/records.rs`
- **Files modified**: `crates/altium-format/src/lib.rs`
- **Verification**: `cargo check -p altium-format` passes, `cargo test -p altium-format -- sch` — 7/7 tests pass
- **Summary**: Defined parsing traits `FromParams`, `ToParams`, `FromBinary`, `ToBinary`. Implemented `SchRecord` enum with `Component(SchComponent)` and `Pin(SchPin)` stubs. `SchRecord::from_block` dispatches text blocks via RECORD parameter (RECORD=0 sentinel → `Ok(None)` with exhaustion check, RECORD=254 → RECORDEX extension, RECORD=1 → Component) and binary blocks via code byte (0x02 → Pin). Exhaustion enforced at dispatch boundary, not inside trait impls. Stubs return `Err(UnknownRecordType/UnknownBinaryCode)` to drive the red/green loop. Seven inline unit tests: sentinel, no-record, unknown type, component stub, pin stub, unknown binary code, sentinel with extra params.
