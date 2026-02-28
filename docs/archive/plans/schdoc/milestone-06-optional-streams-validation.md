# Milestone 6: Optional Streams & Final Validation

Handle optional SchDoc streams intentionally and finalize strict consistency checks.

## Files

- `crates/altium-format/src/schdoc/mod.rs`
- `crates/altium-format/src/schdoc/optional_streams.rs`

## Requirements

- Detect optional streams:
  - `ObjectDefinitions`
  - `ReuseBlockInfos`
  - `ReuseBlocks` / `ReuseBlocksV2`
  - `HarnessConnectionPointConnector`
  - `Files`
- For each optional stream:
  - parse if format is known, or
  - return explicit unsupported error when present but unimplemented
- Run final validations:
  - owner index references must be in range for base/additional lists
  - required sheet/template records are present
  - all tracked streams are consumed

## Acceptance Criteria

- `SchDoc::open` succeeds on fixtures with only standard 3 streams.
- Optional stream presence does not get silently ignored.
- `TrackedCfbDocument::assert_all_consumed()` passes on supported fixtures.

## Tests

- Integration tests for standard SchDoc files.
- Targeted tests with synthetic optional stream presence to verify explicit behavior.

