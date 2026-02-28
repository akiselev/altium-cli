# Milestone 4: Storage Stream Integration

Parse `/Storage` and merge embedded objects into image records.

## Files

- `crates/altium-format/src/schdoc/storage.rs`
- `crates/altium-format/src/schdoc/mod.rs`

## Requirements

- Parse `/Storage` header (`HEADER=Icon storage`, `Weight=N`).
- Parse binary entry blocks (flags `0x01`) with embedded object envelope (`0xD0` tag).
- Decompress zlib payload and store with object id/path.
- Link embedded objects to `SchImage` records via filename/id match.
- Error if referenced image record cannot be resolved when `EmbedImage` requires payload.

## Acceptance Criteria

- Embedded object count matches storage stream `Weight`.
- Invalid tag/compression errors include stream and entry index context.
- Linked `SchImage` entries expose decoded binary payload in the SchDoc model.

## Tests

- Integration test with SchDoc fixture containing embedded images.
- Error case tests for malformed object tag and corrupt compressed data.

