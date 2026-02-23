# Milestone 1: Module Foundation

Create the SchDoc module structure under `crates/altium-format/src/schdoc/` and replace
the current stub implementation with a typed document skeleton.

## Files

- `crates/altium-format/src/schdoc.rs` -> `crates/altium-format/src/schdoc/mod.rs`
- `crates/altium-format/src/schdoc/types.rs`
- `crates/altium-format/src/lib.rs` (module wiring)

## Requirements

- Convert SchDoc from single-file stub to directory module.
- Define `SchDoc` core struct with fields for:
  - header metadata (header string, weight, minor version, unique id)
  - base record list
  - additional record list
  - embedded storage objects
- Keep internals `pub(crate)` where possible; only public API is `SchDoc::open(...)`.
- Keep `SchDoc::open` fallible and wired to `TrackedCfbDocument`.

## Acceptance Criteria

- `cargo build` passes.
- Existing imports (`pub use schdoc::SchDoc`) continue to compile.
- No regression for other document modules.

## Tests

- Unit compile tests in `schdoc/mod.rs` for struct construction.
- Smoke test: opening a known `.SchDoc` reaches CFB open path without panic.

