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

