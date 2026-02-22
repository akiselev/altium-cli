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
