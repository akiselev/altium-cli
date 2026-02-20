# altium-cli

Rust workspace for reading, writing, and querying Altium Designer files.

## Workspace Structure


* **altium-format-derive** Procedural macros for serialization code generation
* **altium-format**  Core library for Altium file parsing and manipulation
* **altium-cli**  Command-line tool for file inspection and manipulation

## Architecture

Three-crate dependency graph ensures clean separation:

```
altium-format-derive (proc macros, no runtime deps)
     ↓
altium-format (core library: parsing, querying, editing)
     ↓
altium-cli (binary: CLI interface, output formatting)
```

**Publishing order:** derive → format → cli (format depends on derive, cli depends on format).

**Versioning:** Synchronized versions (all crates at same version for initial releases).




# Reverse engineering Altium

If you are developing locally on the dev's machine, you can use ghidra-cli (project: altium26) to reverse engineer the Delphi DLLs that handle the file formats (list binaries on the project to see which ones are available) and you can see the decompiled C# source code for the dotnet code in AD26-dotnet/ (it's millions of lines so you'll need to use ripgrep or similar)

When working on unimplemented record types, make sure to reference both the C# code and Delphi code to make sure you have full support.


# Privacy

The altium-format implementation details like ParamCollection, the FromOrigin/ToOrigin traits, and so on, MUST BE KEPT PRIVATE TO THE CRATE. THEY ARE IMPLEMENTATION DETAILS THAT MUST NOT BE EXPOSED TO THE OPS CRATE.

The ENTIRE POINT OF THE REBUILD COMMANDS AND ROUND TRIPPING IS TO MAKE SURE OUR FILE FORMAT IS COMPLETE AND FULLY SUPPORTS THE ALTIUM FILE FORMAT. AVOID SILENTLY USING DEFAULTS AND UNWRAP_OR AND STUFF LIKE THAT. IF AN OPERATION CAN FAIL, THAT FUNCTION MUST ALWAYS RETURN Result<T, AltiumError>