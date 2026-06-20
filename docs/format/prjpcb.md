# PrjPcb Files

`.PrjPcb` is a UTF-8 INI-style text format, not CFB. Files may begin with a UTF-8 BOM; the writer emits one.

The parser in [`project.rs`](../../crates/altium-format/src/project.rs):

- reads section headers such as `[Design]`, `[DocumentN]`, and `[OutputGroupN]`;
- splits key/value lines on the first `=`;
- preserves ordering with `IndexMap`;
- maps supported sections into typed public API structures;
- ignores comment and blank lines.

Current support is summarized in [`STATUS.md`](../../STATUS.md). The internal serializer supports roundtrip writing; the high-level public API remains read-only.

