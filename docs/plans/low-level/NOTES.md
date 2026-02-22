# Low-Level API Implementation Notes

## Milestone 1: Error Types Expansion

- No existing code referenced `InvalidParamValue(String)` or `BinaryParsingError(String)` outside the enum definition, so the signature changes were non-breaking in practice.
- Error variant display strings match the design doc exactly (e.g., `"Unknown binary code in schematic block: 0x{0:02X}"` with hex formatting).
- `UnknownRecordType(i32)` kept unchanged since it was already in the correct form.
- All variants use `thiserror` `#[error(...)]` for `Display` impl — no manual `Display` implementations needed.

## Milestone 2: Layer 1 — CfbDocument

- Tests placed as inline unit tests (`#[cfg(test)] mod tests` inside `cfb_document.rs`) rather than integration tests in `tests/cfb_document.rs` — `CfbDocument` is `pub(crate)` and inaccessible from a separate test crate.
- Manual `Debug` impl on `CfbDocument` because `cfb::CompoundFile` does not derive `Debug`, which is needed for `assert!(matches!(...))` panic messages in tests.
- `#[allow(dead_code)]` on `mod cfb_document` in `lib.rs` — methods like `exists` and `list_entries` are not yet called but are designed for use by `TrackedCfbDocument` (Layer 2, Milestone 3).
- `enumerate_all_entries` uses recursive walk via `read_storage` — collects children into a `Vec` first to release the borrow on `self.inner` before recursing.
- Test data paths use `env!("CARGO_MANIFEST_DIR")` + `../../data/` to reach workspace-level `data/` directory.
- `enumerate_all_entries` test asserts 4 specific known paths from `BlankSchlibComponent.SchLib`: `/FileHeader`, `/Storage`, `/Component_1`, `/Component_1/Data` — the nested path proves recursive descent works.

## Milestone 3: Layer 2 — TrackedCfbDocument

- Tests placed as inline unit tests (`#[cfg(test)] mod tests` inside `tracked_cfb.rs`) rather than integration tests — same reasoning as Milestone 2: `TrackedCfbDocument` is `pub(crate)` and inaccessible from a separate test crate.
- `list_entries` path normalization uses explicit `"/"` guard before `trim_end_matches('/')` — without the guard, `"/"` would normalize to `""` (empty string), causing `cfb::read_storage("")` to receive a malformed path.
- `read_stream` marks path as consumed before delegating to inner — if the delegate fails, the path remains in `consumed`, but this is benign because errors propagate via `?` and `assert_all_consumed()` is only reached on the success path.
- `read_stream_optional` marks consumed even for absent paths — inserting extra entries into `consumed` that aren't in `all_entries` is harmless since `assert_all_consumed` uses `all_entries.difference(&consumed)`.
- `UnconsumedStreams` error variant already existed in `lib.rs` from Milestone 1 — no addition needed.
- `#[allow(dead_code)]` on `mod tracked_cfb;` in `lib.rs` — `TrackedCfbDocument` has no callers yet; will be consumed by higher layers in later milestones.
- Test for full consumption dynamically discovers entries via `list_entries("/")` rather than hardcoding — `list_entries` returns bare names (e.g., `"FileHeader"`), full paths constructed by prepending parent path (e.g., `format!("/{name}")`).

## Milestone 4: Layer 3 — Block Stream Parser

- Tests placed as inline unit tests inside `block_stream.rs` — same reasoning as previous milestones: `pub(crate)` types inaccessible from integration tests.
- `BlockIter` is a separate struct rather than returning `impl Iterator` — allows callers to hold the iterator in struct fields.
- The `parse_blocks` function delegates to `BlockIter` internally — single implementation, two calling conventions (eager vs lazy).
- Arithmetic right-shift of i32 by 24 leaves sign-extended bits 24-31 in the low byte — casting to u8 truncates correctly regardless of sign extension.
- `#[allow(dead_code)]` on `mod block_stream;` in `lib.rs` — no callers yet; will be consumed by `embedded_object.rs` and `sch/records.rs`.

## Milestone 5: Layer 4 — BinaryReader and BinaryWriter

- Tests placed as inline unit tests inside `binary_io.rs`.
- **Plan correction: Real48 mantissa bit mapping** — the plan's algorithm used `ieee_mant << 1` then extracted via `>> 44` etc., which loses the MSB of the IEEE mantissa when it overflows the `u8` truncation boundary. Corrected to extract directly: `(ieee_mant >> 45)` for byte 5, `>> 37` for byte 4, etc. Read corrected from `raw39 << 12 >> 1` to `raw39 << 13` to align Real48's 39-bit mantissa with IEEE's 52-bit mantissa (MSB of Real48 maps to bit 51 of IEEE).
- **Plan correction: Real48 exponent arithmetic** — the plan used `(exponent as u64 - 129 + 1023)` which overflows in Rust debug mode when exponent < 129. Corrected to `((exponent as i64 - 129 + 1023) as u64)` for safe signed arithmetic.
- `read_bool` uses `!= 0` rather than `== 1` — Delphi Boolean convention where any non-zero byte is `true`.
- `sub_reader` advances the parent reader by exactly `len` bytes — the sub-reader gets an independent slice, and parent position jumps past the reserved region.
- `check_available` is private (`fn`, not `pub(crate) fn`) — callers use the read methods which call it internally.
- `write_pascal_string` panics (not `Result`) if encoded string exceeds 255 bytes — write-side signatures are all infallible per the plan.

## Milestone 6: Layer 4 — ParameterCollection

- Tests placed as inline unit tests inside `param_collection.rs`.
- **Plan correction: 0x8E escape character** — the plan's `unescape_param_value` used `\u{008E}` (Unicode control character), but Windows-1252 byte 0x8E actually decodes to U+017D (Ž, Latin Capital Letter Z with Caron). Corrected to use `\u{017D}` in all escape/unescape logic. Similarly, double 0x8E produces literal Ž (U+017D), not U+008E.
- `from_bytes` splits on raw bytes (`0x7C`) before decoding — this preserves `%UTF8%` value integrity since UTF-8 bytes might contain 0x7C-equivalent sequences that shouldn't be treated as delimiters.
- `from_str_params` is a private helper called by both `from_bytes` (after Windows-1252 decode) and `from_utf16le_bytes` (after UTF-16LE decode) — `%UTF8%` handling only applies to the raw-byte path.
- `shift_remove` used instead of `swap_remove` — preserves insertion order for remaining keys, which matters for round-trip serialization fidelity.
- Case-insensitive lookup via `to_ascii_lowercase()` comparison — `IndexMap` keys are stored in original case, so lookup iterates all keys for each access. This is O(n) per lookup but ParameterCollections are small (typically <50 keys).
- `ToParamValue` import unused warning — the trait is defined but not yet consumed; will be used when serialization is implemented.

## Milestone 7: Layer 4 — Embedded Object Envelope Parser

- Tests placed as inline unit tests inside `embedded_object.rs`.
- `#[derive(Debug)]` added to `EmbeddedObject` — required for `assert!(matches!(...))` and `unwrap_err()` in tests, not explicitly in plan but necessary for compilation.
- `parse_embedded_object_stream` consumes RECORD and Weight from header block internally — returns only `Vec<EmbeddedObject>`, never a live `ParameterCollection`. This prevents callers from accidentally skipping exhaustion checks on the header.
- `parse_embedded_object` calls `reader.assert_exhausted()` after reading inner data — any trailing bytes in the envelope are a hard error.
- Weight validation happens after all entries are parsed — this gives better error messages (exact actual count vs expected).

## Milestone 8: Layer 5 — Parsing Traits and SchRecord Enum Scaffold

- Tests placed as inline unit tests inside `sch/records.rs`.
- `SchComponent` and `SchPin` stubs return `Err` from their trait impls — this is intentional for the red/green development loop. The first real SchLib file parsed will hit `UnknownRecordType(1)`, signaling that Component fields need implementation.
- `assert_exhausted` called at the dispatch boundary in `SchRecord::from_block`, not inside `FromParams`/`FromBinary` impls — because base types and record-specific types share one `ParameterCollection` (flatten pattern), an exhaustion check inside a base type would reject the record-specific fields that follow.
- RECORD=254 extension: effective_id comes from RECORDEX when RECORD=254, supporting record IDs >253 without breaking the 1-byte RECORD encoding.
- RECORD=0 sentinel with extra params fails exhaustion — test verifies that even sentinel blocks enforce fail-fast on unknown parameters.
- `#[allow(dead_code)]` on `mod sch;` in `lib.rs` — module has no external callers yet; will be consumed by `SchLib::open` in Milestone 9.
- Unused import warnings on `sch/mod.rs` re-exports — `FromBinary`, `FromParams`, `SchRecord`, `ToBinary`, `ToParams` are re-exported for convenience but not yet used outside `sch/records.rs`; suppressed via `#[allow(dead_code)]` on the module.
