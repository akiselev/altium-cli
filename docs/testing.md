# Testing and Validation

Default tests do not read fixture repositories or run property tests:

```bash
cargo test --workspace
```

Fixture and property suites are opt-in and must not be run unless explicitly requested:

```bash
cargo test --workspace --features test-fixtures
cargo test --workspace --features proptest
```

Use targeted tests during development:

```bash
cargo test -p altium-format <test-name>
cargo test -p altium-format-spec <test-name>
```

For parser validation, use the CLI against a specific file:

```bash
altium validate path/to/file
altium save-as original.PcbLib roundtrip.PcbLib
altium cfb diff --semantic original.PcbLib roundtrip.PcbLib
```

Semantic CFB comparison is implemented in [`test_utils.rs`](../crates/altium-format/src/test_utils.rs). It compares entries, block framing, parameter pairs, binary blocks, and decompressed embedded objects while preserving fail-fast behavior.

After structural edits, run invariant checks and save/reopen validation. Never weaken a test to accept unknown or unconsumed data.

