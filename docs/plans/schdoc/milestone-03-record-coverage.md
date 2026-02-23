# Milestone 3: SchDoc Record Coverage

Extend record parsing to include SchDoc-only records while reusing shared SchLib/SchDoc
record structs where formats are identical.

## Files

- `crates/altium-format/src/sch_records.rs`
- `crates/altium-format/src/schdoc/dispatch.rs`

## Requirements

- Keep existing shared record parsing for:
  - component/pin/label/polyline/polygon/arc/rectangle/line/image/text/implementation records
- Add SchDoc-only record support:
  - `RECORD=31` Sheet
  - `RECORD=39` Template
  - `RECORD=27` Wire
  - `RECORD=26` Bus
  - `RECORD=25` NetLabel
  - `RECORD=17` PowerObject
  - `RECORD=18` Port
  - `RECORD=22` NoConnect
  - `RECORD=29` Junction
  - `RECORD=15` SheetSymbol
  - `RECORD=16` SheetEntry
  - `RECORD=43` ParameterSet
  - `RECORD=209` Note
  - `RECORD=210` Probe
  - `RECORD=211` CompileMask
  - `RECORD=225` (for Additional stream path)
- Ensure SchDoc pins (`RECORD=2`) parse from parameter text path, not binary pin parser.

## Acceptance Criteria

- Dispatch covers all currently documented SchDoc records required by milestone scope.
- Unknown keys in implemented record types fail via exhaustion checks.
- Pin parsing path is selected by stream format/context (text in SchDoc FileHeader).

## Tests

- Record-level unit tests for each new SchDoc-only struct using synthetic parameter blocks.
- Integration parse against SchDoc corpus to verify no immediate unknown-record failures.

