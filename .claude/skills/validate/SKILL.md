# Validate Skill

Validate, roundtrip-test, and debug Altium files (SchLib, PcbLib, SchDoc, PcbDoc, IntLib).

## CLI Commands

### Validate

Parse file and check invariants. Fails on first unknown record/parameter/stream.

```bash
altium validate <file>          # .schdoc .schlib .pcbdoc .pcblib .intlib .prjpcb
```

Exit 0 = pass. Exit 1 = error with full context chain.

### Save-As (Roundtrip)

Open, re-serialize, and save. Use with `cfb diff --semantic` to find serialization bugs.

```bash
altium save-as input.SchLib output.SchLib
# Supported: .schdoc .schlib .pcblib (PcbDoc not yet)
```

### CFB Inspection

```bash
altium cfb ls <file> [--flat]                    # List streams/storages
altium cfb dump <file> <stream> [--blocks]       # Hex+ASCII dump
altium cfb blocks <file> <stream> [--block N]    # Block summary or detail
altium cfb cat <file> <stream>                   # Raw bytes to stdout
altium cfb diff <f1> <f2> [--blocks] [--stream S] [--verbose]  # Byte-level diff
```

### Semantic Diff

Order-agnostic parameter comparison, embedded object decompression, block-aware.

```bash
altium cfb diff --semantic <f1> <f2>                   # Categorized report
altium cfb diff --semantic --verbose <f1> <f2>         # Flat numbered list
altium cfb diff --semantic --stream /FileHeader <f1> <f2>  # Single stream
```

Issue categories (fix in this priority order):
1. `EntryMissingInB` - sidecar streams not written
2. `MissingParamPair` - parameters dropped during save
3. `UpdatedParamValues` - formatting differences (e.g. `0mil` vs `0.0000mil`)
4. `BinaryBlockMismatch` - binary record serialization errors
5. `BlockParseError` + `RawByteMismatch` - raw binary stream differences

## Roundtrip Debugging Workflow

```bash
# 1. Save roundtrip copy
altium save-as original.SchLib roundtripped.SchLib

# 2. Semantic diff (recommended first step)
altium cfb diff --semantic original.SchLib roundtripped.SchLib

# 3. Verbose for full issue list
altium cfb diff --semantic --verbose original.SchLib roundtripped.SchLib

# 4. Inspect specific stream
altium cfb blocks original.SchLib /Component_1/Data --block 0

# 5. Raw hex for external tools
altium cfb cat original.SchLib /FileHeader | xxd
```

## Red/Green Development Loop

The parser fails on any unknown data. Use `altium validate` in a loop to implement format support incrementally:

```bash
altium validate file.PcbLib
# Error: Unknown object ID 0x07 at...
# → Investigate via C# (AD26-dotnet/) and Delphi (ghidra altium26)
# → Implement typed parse + serialize
# → Re-run validate
```

## Tests

### Feature Flags

| Feature         | Gates                                    | Speed     |
|-----------------|------------------------------------------|-----------|
| (none)          | Unit tests only                          | Fast      |
| `test-fixtures` | Tests reading files from `data/`         | Medium    |
| `proptest`      | Property-based tests (implies fixtures)  | Slow      |

### Running Tests

```bash
cargo test --workspace                          # Unit tests only
cargo test --workspace --features test-fixtures  # + fixture tests
cargo test --workspace --features proptest       # + proptests

# Targeted runs during development
cargo test -p altium-format <test_name>
cargo test -p altium-format-ops <test_name> --features test-fixtures
```

### Gating New Tests

```rust
// No feature flag needed - pure unit test
#[test]
fn test_parse_logic() { ... }

// Reads from data/ - MUST gate
#[cfg(feature = "test-fixtures")]
#[test]
fn test_fixture_roundtrip() {
    let path = schlib_fixture_path("Resistors_Caps.SchLib");
    let lib = SchLib::open(&path).expect("open");
    lib.validate_invariants().expect("valid");
}

// Proptest - MUST gate
#[cfg(feature = "proptest")]
proptest! { ... }
```

### Test Utilities (`altium-format::test_utils`)

```rust
use crate::test_utils::{assert_cfb_files_semantic_eq, diff_cfb_files_semantic};

// Panic with detailed report if semantically different
assert_cfb_files_semantic_eq(original_path, roundtripped_path);

// Get report for inspection
let report = diff_cfb_files_semantic(path_a, path_b)?;
if !report.is_identical() {
    eprintln!("{}", report.render_categorized());
}
```

### Integration Test Harness (`altium-format-ops/tests/harness/`)

```rust
use harness::{schlib_fixture_path, pcblib_fixture_path, schdoc_fixture_path, pcbdoc_fixture_path};
use harness::{save_reopen_schlib, save_reopen_schdoc, validate_pcbdoc};
```

### Test Fixture Repositories

Clone into `data/` if missing:

| Directory      | Repository                                         |
|----------------|-----------------------------------------------------|
| `data/schlib/` | https://github.com/akiselev/altium-cli-test-schlib  |
| `data/pcblib/` | https://github.com/akiselev/altium-cli-test-pcblib  |
| `data/intlib/` | https://github.com/akiselev/altium-cli-test-intlib  |
| `data/schdoc/` | https://github.com/akiselev/altium-cli-test-schdoc  |
| `data/pcbdoc/` | https://github.com/akiselev/altium-cli-test-pcbdoc  |

Additional fixtures in `data/` root: `BlankSchlibComponent.SchLib`, `BlankPcbLibComponent.PcbLib`, `LimeMicroAltiumLib_*.SchLib/.PcbLib`, etc.

### Proptest Regression Seeds

Located at:
- `crates/altium-format/proptest-regressions/`
- `crates/altium-format-ops/proptest-regressions/`
- `crates/altium-format-ops/tests/*.proptest-regressions`

When proptest finds a failure: minimize the seed, commit regression file with the fix.

## Other Useful Commands

```bash
altium get version <file>                        # Format version (.schlib/.pcblib)
altium render <file> [-o dir] [--name X] [--format svg|png] [--scale N]
altium new schdoc <output>                       # Create blank SchDoc
altium new schlib <output>                       # Create blank SchLib
altium ops apply <file> --spec-file <ops> [-o path] [--dry-run] [--report-json]
```
