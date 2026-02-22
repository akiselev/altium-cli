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