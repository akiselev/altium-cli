# altium-cli

Rust workspace for reading, writing, and querying Altium Designer files.

## Workspace Structure

| Crate                                                         | What                                                   | When                                             |
| ------------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------ |
| [altium-format-derive](crates/altium-format-derive/CLAUDE.md) | Procedural macros for serialization code generation    | Implementing new record types                    |
| [altium-format](crates/altium-format/CLAUDE.md)               | Core library for Altium file parsing and manipulation  | Using library API, extending file format support |
| [altium-cli](crates/altium-cli/CLAUDE.md)                     | Command-line tool for file inspection and manipulation | Building/using CLI tool                          |

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

## Build Commands

```bash
# Build entire workspace
cargo build --workspace

# Build specific crate
cargo build -p altium-cli
cargo build -p altium-format
cargo build -p altium-format-derive

# Run all tests
cargo test --workspace

# Build CLI binary
cargo run -p altium-cli -- --help

# Verify publishability
cargo publish --dry-run -p altium-format-derive
cargo publish --dry-run -p altium-format
cargo publish --dry-run -p altium-cli

# Generate documentation
cargo doc --workspace --no-deps --open
```

## Documentation

| File | What |
|------|------|
| [SKILL.md](SKILL.md) | Full CLI command reference for AI agents |
| [docs/README.md](docs/README.md) | Documentation index |
| [docs/commands/](docs/commands/) | Detailed command reference by group |

## CI/CD Workflow

Single workflow handles test, build, release, and publish:

| Workflow                      | What                                                   | When           |
| ----------------------------- | ------------------------------------------------------ | -------------- |
| .github/workflows/release.yml | Test → Build → GitHub Release → Publish to crates.io   | Tag push (v*) |

**Workflow data flow:**
1. Tag push triggers workflow
2. Test: Run `cargo test --workspace`
3. Build matrix: Linux x86_64, macOS x86_64/aarch64, Windows x86_64
4. Create GitHub Release with binaries
5. Publish derive → wait for propagation → publish format → wait → publish cli

## Installation

**CLI tool:**
```bash
cargo install altium-cli
```

**Library:**
```toml
[dependencies]
altium-format = "0.1.0"
```

## Quick Start

**CLI usage:**
```bash
# Inspect file structure
altium-cli inspect components.SchLib

# Query components
altium-cli query design.SchDoc "R*"

# Schematic analysis
altium-cli schdoc bom design.SchDoc
altium-cli schdoc netlist design.SchDoc

# PCB analysis
altium-cli pcbdoc rules design.PcbDoc
altium-cli pcbdoc components design.PcbDoc

# Project management
altium-cli prjpcb overview project.PrjPcb
altium-cli prjpcb bom project.PrjPcb --grouped

# Library browsing
altium-cli schlib list components.SchLib
altium-cli pcblib measure footprints.PcbLib SOIC-8
altium-cli intlib search library.IntLib "LM358"

# Edit schematic
altium-cli edit design.SchDoc -c "move U1 1000 2000" -o output.SchDoc
```

**Library usage:**
```rust
use altium_format::io::SchLib;
use std::fs::File;
use std::io::BufReader;

let file = File::open("components.SchLib")?;
let lib = SchLib::open(BufReader::new(file))?;
```
