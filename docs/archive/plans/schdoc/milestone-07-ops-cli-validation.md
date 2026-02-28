# Milestone 7: Ops/CLI Validation Integration

Wire SchDoc parsing into `altium-format-ops` validation and CLI execution paths.

## Files

- `crates/altium-format-ops/src/schdoc_ops.rs`
- `crates/altium-cli/src/main.rs`
- SchDoc test modules in `altium-format` and `altium-format-ops`

## Requirements

- Replace `SchDocOps::validate` stub with real checks over parsed SchDoc model.
- Ensure CLI `validate` command for `.SchDoc` uses implemented validation path and surfaces context-rich errors.
- Add a SchDoc fixture suite for regression:
  - minimal file
  - file with embedded storage images
  - file with Additional records

## Acceptance Criteria

- `altium validate <file.SchDoc>` succeeds for supported fixtures.
- Invalid fixtures fail with actionable error context (stream + record index/type).
- No unimplemented error returned for SchDocOps validate.

## Tests

- Ops-level validation tests for passing and failing SchDoc fixtures.
- CLI smoke tests (if existing harness supports SchDoc command execution).

