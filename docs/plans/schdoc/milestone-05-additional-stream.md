# Milestone 5: Additional Stream Integration

Parse `/Additional` and add supplementary records to a separate Additional warehouse list.

## Files

- `crates/altium-format/src/schdoc/additional.rs`
- `crates/altium-format/src/schdoc/mod.rs`

## Requirements

- Parse `/Additional` header with optional `Weight`.
- Parse `Weight` records when present; support `RECORD=225` dashed rectangle records.
- Keep Additional records separate from base FileHeader records in the in-memory model.
- Preserve ownership semantics for `OWNERINDEXADDITIONALLIST`.

## Acceptance Criteria

- Header-only Additional stream (`Weight` absent/zero) parses cleanly.
- Non-zero `Weight` is enforced against actual parsed record count.
- Unsupported record type in Additional stream fails fast with explicit error.

## Tests

- Fixture with no Additional records.
- Fixture with multiple `RECORD=225` entries.
- Count mismatch negative test.

