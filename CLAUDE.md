# altium-cli

Rust workspace for reading, writing, and querying Altium Designer files.

## Workspace Structure


* **crates/altium-format-derive** Derive macros for serialization code generation
* **crates/altium-format-ops**  High level operations for manipulating Altium files (used by altium-cli)
* **crates/altium-format**  Core library for Altium file parsing and manipulation
* **crates/altium-cli**  Command-line tool for file inspection and manipulation

## Architecture

Three-crate dependency graph ensures clean separation:

```
altium-format-types (core types from Altium like constants, enums, and structs)
     ↓
altium-format-derive (proc macros, no runtime deps)
     ↓
altium-format (core library: parsing, querying, editing)
     ↓
altium-format-ops (high level operations like summaries, add, edit, etc.)
     ↓
altium-cli (binary: CLI interface, output formatting)
```

**Publishing order:** derive → format → ops → cli (format depends on derive, ops depends on format, cli depends on ops).

**Versioning:** Synchronized versions (all crates at same version for initial releases).

**Design Philosophy**: Fail fast, fail hard. No round-trip preservation, no unknown field
capture, no opaque blobs. If our parser encounters data it doesn't understand, that is a
bug in our code that must be fixed -- never silently skipped. These files control PCB
fabrication; a silently dropped field could cost thousands of dollars.

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

The altium-format implementation details MUST BE KEPT PRIVATE TO THE CRATE. THEY ARE IMPLEMENTATION DETAILS THAT MUST NOT BE EXPOSED TO THE OPS CRATE.

We MUST NEVER silently drop parsing or other errors or silently corrupt data. Everything that is fallible, MUST RETURN A Result<T, AltiumFormatError>


# Error Handling

* altium-format uses altium-format::AltiumFormatError
* altium-format-ops uses altium-format-ops::AltiumOpsError
* altium-cli uses anyhow

# Red/green development

We are using a red/green development workflow similar to red/green test driven development except along with tests we are using our own validate CLI command to open documents. Since we fail on the first record/type/parameter that we don't recognize, Claude Code can use the command in a loop to slowly investigate and implement every part of the Altium file format step by step.


# CFB Debugging CLI

The `altium cfb` subcommand group provides low-level CFB container inspection tools
for debugging serialization roundtrip failures. These operate directly on the CFB
container using the `cfb` crate — no `altium-format` imports.

| Command | Purpose |
|---------|---------|
| `altium cfb ls <file>` | List streams/storages (tree view, or `--flat` for grep-friendly) |
| `altium cfb dump <file> <stream>` | Hex+ASCII dump (`--blocks` annotates block boundaries and decodes text) |
| `altium cfb blocks <file> <stream>` | Block-level summary (`--block N` for full detail on one block) |
| `altium cfb diff <file1> <file2>` | Stream-by-stream comparison (`--blocks` for block-level, `--stream` to filter) |
| `altium cfb cat <file> <stream>` | Raw bytes to stdout for piping (`\| xxd`, `\| wc -c`) |

Example workflow for debugging a roundtrip failure:
```bash
# 1. Diff original vs roundtripped
altium cfb diff original.SchLib roundtripped.SchLib --blocks

# 2. Inspect the differing stream
altium cfb blocks original.SchLib /Component_1/Data
altium cfb blocks original.SchLib /Component_1/Data --block 0

# 3. Hex dump with block annotations
altium cfb dump original.SchLib /FileHeader --blocks

# 4. Pipe raw bytes for external tools
altium cfb cat original.SchLib /FileHeader | xxd
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
- **Parameter keys**: Case-insensitive, first occurrence wins. `%UTF8%` prefix for Unicode keys. `[]` escapes `|`, `{}` escapes `=`