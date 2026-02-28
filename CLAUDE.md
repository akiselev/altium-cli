# altium-cli

Rust workspace for reading, writing, and querying Altium Designer files.

## Workspace Structure


* **crates/altium-format-derive** Derive macros for serialization code generation
* **crates/altium-format-types** Raw types types reverse engineered from Altium
* **crates/altium-format**  Core library for Altium file parsing and manipulation
* **crates/altium-format-spec**  Spec language compiler, executor, and reconciler
* **crates/altium-cli**  Command-line tool for file inspection and manipulation

## Architecture

Dependency graph:

```
altium-format-types (core types from Altium like constants, enums, and structs)
     ↓
altium-format-derive (proc macros, no runtime deps)
     ↓
altium-format (core library: parsing, querying, editing)
     ↓
altium-format-spec (spec language: compiler, executor, reconciler)
     ↓
altium-cli (binary: CLI interface, output formatting)
```

**Publishing order:** types → derive → format → spec → cli.

**Versioning:** Synchronized versions (all crates at same version for initial releases).

**Design Philosophy**: Fail fast, fail hard. No round-trip preservation, no unknown field
capture, no opaque blobs. If our parser encounters data it doesn't understand, that is a
bug in our code that must be fixed -- never silently skipped. These files control PCB
fabrication; a silently dropped field could cost thousands of dollars.

## CARDINAL RULE: NEVER RETAIN OPAQUE FORMAT DATA

This rule is non-negotiable and overrides convenience during implementation:

- NEVER add "opaque"/"raw" retention fields for format sections or sidecars
  (examples of forbidden patterns: `opaque_sidecars`, `raw_*`, `unknown_bytes`,
  `Vec<u8>` placeholders used to carry undecoded file-format content).
- NEVER "temporarily" keep sidecar bytes for later reverse engineering.
- NEVER bypass this by claiming "source-backed save" if bytes are not fully typed and
  semantically represented in the in-memory model.
- If a stream/sidecar exists and is not understood:
  1. reverse engineer it now (C#/Delphi + fixture analysis), and
  2. implement typed parse + typed serialize, or
  3. return a hard error with full context.
- "Parse later" placeholders are forbidden in merged code.

Pre-merge self-check for agents:
- Search touched code for forbidden escape hatches:
  `opaque|raw_payload|unknown_bytes|unparsed|todo.*sidecar|Custom.*opaque|retain.*bytes`
- If found, remove by implementing typed support or restoring hard fail-fast behavior.

**NEVER skip or suppress unconsumed data**: Do NOT mark streams, records, or fields as
"consumed" without actually parsing them. This includes any form of "skip_known",
"ignore_remaining", or marking entries as consumed in `TrackedCfbDocument` without reading
and parsing their contents. If a stream or field exists in the file, the parser MUST either
fully parse it or return an error. Deferring parsing to "future milestones" by silently
suppressing errors is forbidden — it masks bugs and violates the fail-fast invariant.
`assert_all_consumed()` exists precisely to catch unhandled data; circumventing it defeats
the entire safety model.

**Use domain types from `altium-format-types`**: The `altium-format-types` crate defines typed
enums and structs for every Altium concept (`PcbObjectId`, `SchRecordType`, `Color`, `Coord`,
`UniqueId`, etc.) as well as named constants for format-level values (tag bytes, flag values,
type codes, masks, shifts). ALWAYS use these instead of raw primitives:
- Struct fields: `PcbObjectId` not `u8`, `SchRecordType` not `i32`, `Coord` not `i32`, etc.
- Constants: `INSTRUCTION_BINARY` not `0xD0`, `BLOCK_SIZE_MASK` not `0x00FF_FFFF`, etc.
- If a type or constant doesn't exist yet, add it to `altium-format-types` before using it.
  Types go in the appropriate module (`pcb.rs`, `sch.rs`, etc.); constants go in
  `crates/altium-format-types/src/constants/`. Make sure to check the constant you add against the decompiled code (Delphi or C# depending on the constant, but most should be in the already decompiled C# code)

NEVER use raw primitive integers where a domain type exists (e.g., use `Coord` not `i32`,
`SchRecordType` not `i32`, `PcbObjectId` not `u8`). If a type doesn't already exist in
`altium-format-types`, add one (discuss it with the user first).

**Strings:** Rust `String` (UTF-8) is the correct in-memory representation for decoded text.
Altium files use multiple encodings (Windows-1252 for parameter strings, UTF-8 via `%UTF8%`
prefix, UTF-16LE in pin sidecars and WideStrings). The encoding is a property of the
*serialization context*, not the string type — Altium's own C# code uses plain .NET `string`.
All encoding/decoding MUST happen at parse boundaries with strict error checking:
- Windows-1252: `encoding_rs::WINDOWS_1252.decode()` (all 256 byte values are valid, cannot error)
- UTF-16LE: `encoding_rs::UTF_16LE.decode()` — MUST check `had_errors` flag
- UTF-8: `std::str::from_utf8()` (strict) — NEVER use `from_utf8_lossy()`



# Reverse engineering Altium

If you are developing locally on the dev's machine, you can use ghidra-cli (project: altium26) to reverse engineer the Delphi DLLs that handle the file formats (list binaries on the project to see which ones are available) and you can see the decompiled C# source code for the dotnet code in AD26-dotnet/ (it's millions of lines so you'll need to use ripgrep or similar)

When working on unimplemented record types, make sure to reference both the C# code and Delphi code to make sure you have full support.

The entire Altium file format is described via constants in `./AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatConsts.cs` which have been grouped into modules in altium_format_types::constants::*. Make sure you use those constants rather than hard coding values and use the constants to check that ALL file format features, streams, primitives, and records are implemented.


# Privacy

The altium-format implementation details MUST BE KEPT PRIVATE TO THE CRATE. THEY ARE IMPLEMENTATION DETAILS THAT MUST NOT BE EXPOSED TO DOWNSTREAM CRATES.

We MUST NEVER silently drop parsing or other errors or silently corrupt data. Everything that is fallible, MUST RETURN A Result<T, AltiumFormatError>


# Error Handling

* altium-format uses altium-format::AltiumFormatError
* altium-format-spec uses altium-format-spec::SpecError
* altium-cli uses anyhow

**Error context is mandatory**: Every fallible call at a parsing boundary MUST use
`.context()` or `.with_context()` (from `crate::ResultExt`) to attach location
information. An error like "Binary read past end: needed 2 bytes at offset 2" is
useless without knowing WHICH stream, WHICH footprint, WHICH primitive it came from.
Wrap errors at each layer:
- Top-level: footprint name and CFB key (e.g. `"loading footprint 'C0805' (/C0805)"`)
- Stream level: stream path (e.g. `"parsing /C0805/Data"`)
- Record level: record index, type, and offset (e.g. `"primitive #0 (Pad) at offset 0xA"`)

This produces chained errors like:
```
loading footprint 'C0805' (/C0805): parsing /C0805/Data: primitive #0 (Pad) at Data offset 0xA (payload 2 bytes): Binary read past end: needed 2 bytes at offset 2, only 0 remain
```

# Red/green development

We are using a red/green development workflow similar to red/green test driven development except along with tests we are using our own validate CLI command to open documents. Since we fail on the first record/type/parameter that we don't recognize, Claude Code can use the command in a loop to slowly investigate and implement every part of the Altium file format step by step.


# CFB Debugging CLI

The `altium cfb` subcommand group provides low-level CFB container inspection tools
for debugging serialization roundtrip failures. These operate directly on the CFB
container using the `cfb` crate — no `altium-format` imports.

| Command                                | Purpose                                                                        |
| -------------------------------------- | ------------------------------------------------------------------------------ |
| `altium cfb ls <file>`                 | List streams/storages (tree view, or `--flat` for grep-friendly)               |
| `altium cfb dump <file> <stream>`      | Hex+ASCII dump (`--blocks` annotates block boundaries and decodes text)        |
| `altium cfb blocks <file> <stream>`    | Block-level summary (`--block N` for full detail on one block)                 |
| `altium cfb diff <file1> <file2>`      | Stream-by-stream comparison (`--blocks` for block-level, `--stream` to filter) |
| `altium cfb diff --semantic <f1> <f2>` | Semantic diff: order-agnostic params, embedded object decompression            |
| `altium cfb cat <file> <stream>`       | Raw bytes to stdout for piping (`\| xxd`, `\| wc -c`)                          |

Example workflow for debugging a roundtrip failure:
```bash
# 1. Semantic diff original vs roundtripped (recommended first step)
altium cfb diff --semantic original.PcbLib roundtripped.PcbLib

# 2. Verbose semantic diff (flat numbered list, no grouping)
altium cfb diff --semantic --verbose original.PcbLib roundtripped.PcbLib

# 3. Raw byte-level diff with block annotations
altium cfb diff original.SchLib roundtripped.SchLib --blocks

# 4. Inspect the differing stream
altium cfb blocks original.SchLib /Component_1/Data
altium cfb blocks original.SchLib /Component_1/Data --block 0

# 5. Hex dump with block annotations
altium cfb dump original.SchLib /FileHeader --blocks

# 6. Pipe raw bytes for external tools
altium cfb cat original.SchLib /FileHeader | xxd
```

### Semantic Diff (`--semantic`)

The semantic diff compares CFB files at a higher level than raw bytes:
- **Text blocks**: Compared as order-agnostic `|KEY=VALUE|` parameter pairs (handles
  reordering, duplicate pairs, UTF-8/Windows-1252 encoding differences)
- **Binary blocks**: Compared byte-for-byte
- **Embedded objects** (`0xD0` envelopes): Compared after zlib decompression (ignores
  compression-level differences)
- **Raw binary streams** (PCB Data/Header): Compared byte-for-byte when block parsing
  fails on both sides identically
- **Non-param text blocks** (e.g. LayerKindMapping version strings): Falls back to byte
  comparison when param parsing fails on both sides

The default output is a **categorized report** grouped by stream path with issue counts.
Use `--verbose` for a flat numbered list. Exit code is 0 if identical, 1 if differences found.

Issue categories in the report:
- `EntryMissingInA`/`EntryMissingInB`: Stream/storage exists in one file but not the other
- `MissingParamPair`: A `KEY=VALUE` pair exists in one side but not the other
- `UpdatedParamValues`: Same key has different values (e.g. numeric formatting `0mil` vs `0.0000mil`)
- `BinaryBlockMismatch`: Binary block bytes differ
- `BlockParseError`: Block header parsing failed (common for PCB raw binary Data streams)
- `StreamLengthMismatch`/`RawByteMismatch`: Raw stream byte-level differences
- `EmbeddedObjectDataMismatch`: Decompressed embedded object payloads differ

**Workflow for fixing roundtrip issues**: Save a file with `altium save-as`, run
`altium cfb diff --semantic original saved`, then fix issues by category priority:
1. `EntryMissingInB` — sidecar streams not being written back (WideStrings, UniqueIDs, etc.)
2. `MissingParamPair` — parameters being dropped during save
3. `UpdatedParamValues` — numeric formatting differences (e.g. `0mil` vs `0.0000mil`)
4. `BinaryBlockMismatch` — binary record serialization differences
5. `BlockParseError` + `RawByteMismatch` — raw binary stream differences (recompression, etc.)

# Test Utilities (Semantic CFB Diff)

`altium-format` exposes a semantic CFB diff module (also used by `altium cfb diff --semantic`):

- `crates/altium-format/src/test_utils.rs`

Primary API:

- `diff_cfb_files_semantic(path_a, path_b) -> Result<CfbSemanticDiffReport>`
- `assert_cfb_files_semantic_eq(path_a, path_b)` (panic with detailed report)
- `CfbSemanticDiffReport::render()` — flat numbered list of all issues
- `CfbSemanticDiffReport::render_categorized()` — grouped by stream with category counts
- `DiffIssue::category_name()` — issue type label for grouping
- `DiffIssue::stream_path()` — stream/entry path the issue pertains to

What it compares:

- Entry set and entry kind (stream vs storage)
- Stream block framing (`parse_blocks`) with strict block-type mismatch detection
- Text blocks as **order-agnostic parameter pairs**
  - missing/added pairs are errors
  - changed values for the same key are errors
  - duplicate identical key/value pairs are tolerated (to handle known Altium duplication bugs)
- Binary blocks byte-for-byte
- Embedded-object envelopes (`0xD0`) in binary blocks are compared in **decompressed form**
  - zlib output-level differences are ignored
  - decompressed payload differences are hard failures

Why: this gives stronger roundtrip diagnostics than raw byte diffs while still preserving
fail-fast behavior and surfacing real format misunderstandings.

Example use in tests:

```rust
use crate::test_utils::assert_cfb_files_semantic_eq;

let tmp = tempfile::NamedTempFile::new().unwrap();
doc.save(tmp.path()).unwrap();
assert_cfb_files_semantic_eq(original_path, tmp.path());
```


# DXP File Format Documentation (`docs/dxp/`)

Reverse-engineered documentation for Altium Designer binary file formats. **Start with `container-format.md` for the big picture**, then dive into the domain you need.

## Navigation Guide

| When you need to...                                                                             | Read this                                                        |
| ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Understand how all Altium files are structured (CFB containers, block encoding, stream layouts) | `container-format.md`                                            |
| Understand coordinates, DXP fractional encoding, or color format                                | `coordinates.md`                                                 |
| Implement or debug a **schematic** record type                                                  | `schematic-records.md` + `file-format-constants.md`              |
| Understand SchDoc/SchLib loading/saving pipelines, OWNERINDEX linking, font tables              | `sch-files.md`                                                   |
| Implement or debug a **PCB** record type                                                        | `pcb-records.md` + `pcb-files.md`                                |
| Understand PCB binary record format, section registry, loading pipeline                         | `pcb-files.md`                                                   |
| Understand sidecar streams (WideStrings, UniqueIDs, pin sidecars, etc.)                         | `sidecar-streams-deep-dive.md`                                   |
| Verify parameter ordering or serialization invariants                                           | `invariants.md`                                                  |
| Look up .NET interface hierarchy for schematic or PCB                                           | `sch-dotnet-model.md` / `pcb-dotnet-model.md`                    |
| Look up Delphi API exports (`SchApi_*` / `PcbApi_*`)                                            | `sch-api-functions.md` / `pcb-api-functions.md`                  |
| Understand .NET↔Delphi COM interop architecture                                                 | `dotnet-delphi.md`                                               |
| Investigate unknown pad binary fields                                                           | `altium-pad-field-analysis.md` → `altium-pad-unknowns-REPORT.md` |
| Look up raw Delphi enums/constants (TObjectId, TLayer, TShape, etc.)                            | `altium-types.md` / `altium-constants.md`                        |

## Critical Format Facts

- **Container**: All files are OLE/CFB V3 compound documents
- **Block encoding**: 4-byte header = `flags(8b) | size(24b)`, then payload. Flag `0x01` = binary mode
- **Schematic records**: Pipe-delimited `|KEY=VALUE|` strings in Windows-1252. Dispatch on `RECORD=N`
- **PCB records**: Binary little-endian structs. Dispatch on `u8` object ID byte
- **Coordinates**: 10,000 internal units = 1 mil. Schematic splits into integer + `_FRAC` params. PCB stores raw i32
- **Colors**: Win32 COLORREF `0x00BBGGRR` (BGR, not RGB)
- **Schematic ownership**: Flat list + `OWNERINDEX` pointing to parent. In SchLib, indices are component-relative
- **PCB ownership**: Separate sections per primitive type. Cross-references via net/component/polygon indices
- **Sidecar streams**: Format-evolution artifacts — supplementary data in separate CFB streams merged at load time. No runtime distinction from core data
- **PcbLib WideStrings pitfall**: Uses parameter-block format, NOT the binary TLV encoding used by PcbDoc's WideStrings6
- **SchLib pin sidecars**: 9 streams per component (PinFrac → PinFunctionData). PinWideText is authoritative over PinDesc
- **RECORD >= 256**: Written as `RECORD=254` + `RECORDEX=<actual_value>`
- **Parameter keys**: Case-insensitive, first occurrence wins. `%UTF8%` prefix for Unicode keys. Byte 0x8E escapes `|` (single = pipe, double = literal 0x8E). `¦` (broken bar) also represents `|` in UTF-16LE/string contexts. `=` is never escaped (parser splits on first `=` only)


# Test files

Test files for each document type can be found in data/<document type>/ but they must be cloned from their respective repositories if they're missing:

* data/schlib/ - https://github.com/akiselev/altium-cli-test-schlib
* data/pcblib/ - https://github.com/akiselev/altium-cli-test-pcblib
* data/intlib/ - https://github.com/akiselev/altium-cli-test-intlib
* data/schdoc/ - https://github.com/akiselev/altium-cli-test-schdoc
* data/pcbdoc/ - https://github.com/akiselev/altium-cli-test-pcbdoc

**NOTE**: Some of these files may be corrupt, use unsupported encoding like windows_1521 and so on.

# Testing Infrastructure (Agent Guidance)

## Test feature flags

Slow and fixture-dependent tests are gated behind cargo features so that
`cargo test` runs fast by default:

| Feature         | What it gates                                          | Implies         |
| --------------- | ------------------------------------------------------ | --------------- |
| `test-fixtures` | Tests that read files from `data/` fixture directories | —               |
| `proptest`      | Property-based (proptest) tests (slow, randomised)     | `test-fixtures` |

These features are defined in `altium-format` and `altium-cli`.

```bash
# Fast unit tests only (default)
cargo test --workspace

# Include fixture-dependent tests
cargo test --workspace --features test-fixtures

# Include everything (fixtures + proptests)
cargo test --workspace --features proptest
```

When adding new tests:
- Tests that **do not** read from `data/` should work without any feature flag.
- Tests that **read fixture files** from `data/` must be gated with
  `#[cfg(feature = "test-fixtures")]`.
- Property tests (`proptest!` blocks) must be gated with
  `#[cfg(feature = "proptest")]`. Helper functions and imports used exclusively
  by proptest blocks should also be gated.

## Current testing layers

1. Unit tests in source modules (`#[cfg(test)]` blocks in `crates/altium-format/src/*`).
2. Property tests (proptest) — behind `--features proptest`:
   - `crates/altium-format/src/schdoc/mod.rs`
   - `crates/altium-format/src/pcblib/mod.rs`
   - `crates/altium-format/src/pcbdoc/mod.rs`
   - `crates/altium-cli/src/main.rs`
3. Regression seeds:
   - `crates/altium-format/proptest-regressions/*`

## Agent rules for tests

- Always prefer fail-fast assertions with explicit context (stream path, record index, opid, selector).
- Validate invariants after structural edits (`validate_invariants()`), and after save/reopen for roundtrip flows.
- For CFB roundtrip assertions, prefer `assert_cfb_files_semantic_eq` over ad-hoc byte-by-byte compares.
- Never weaken tests to hide unknown data. Unknown stream/record/field handling must fail until implemented.
- When proptest finds a failure, keep/minimize the seed and commit/update regression files with the fix.
- Use targeted test runs while developing:
  - `cargo test -p altium-format <test_name>`
  - `cargo test -p altium-format-spec <test_name>`
  then run broader suites before finishing.


# Older versions

We want to support the last few legacy versions of file formats but like Altium we UPGRADE TO THE LATEST FORMAT! When diffing the CFB files, you need to take that into account.



# TESTS

DO NOT RUN text-fixture TESTS AND proptest TESTS UNLESS EXPLICITLY ASKED TO.