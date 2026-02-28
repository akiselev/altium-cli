# Milestone 2: FileHeader Parse & Dispatch Skeleton

Implement FileHeader stream parsing and a minimal text-record dispatch loop.

## Files

- `crates/altium-format/src/schdoc/fileheader.rs`
- `crates/altium-format/src/schdoc/dispatch.rs`
- `crates/altium-format/src/schdoc/mod.rs`

## Requirements

- Parse `/FileHeader` with strict header validation:
  - `HEADER == Protel for Windows - Schematic Capture Binary File Version 5.0`
  - parse `Weight`, `MinorVersion`, `UniqueID`
- Support both documented layouts:
  - separate header block + sheet block
  - combined parameter set where sheet keys may appear with header keys
- Parse blocks 1..N as parameter text records (flags `0x00` only).
- Extract `RECORD`/`RECORDEX` and dispatch through a centralized SchDoc dispatch function.
- Enforce invariants:
  - first content record is `RECORD=31` (Sheet)
  - second content record is `RECORD=39` (Template)

## Acceptance Criteria

- Base record list length matches `Weight`.
- Unknown record type returns `UnknownRecordType` with context.
- Binary block found in `/FileHeader` returns error.
- `assert_exhausted` enforced for parsed records.

## Tests

- Parse at least one small SchDoc fixture end-to-end for FileHeader only.
- Negative tests:
  - wrong header string
  - wrong first/second record type
  - malformed block header

