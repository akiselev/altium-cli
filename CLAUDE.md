# altium-cli

Rust workspace for reading, writing, and querying Altium Designer files.

## Workspace Structure

| Crate | What | When |
|---|---|---|
| [altium-derive](crates/altium-derive/CLAUDE.md) | Procedural macros for serialization code generation | Implementing new record types |
| [altium-format](crates/altium-format/CLAUDE.md) | Core library for Altium file parsing and manipulation | Using library API, extending file format support |
| [altium-cli](crates/altium-cli/CLAUDE.md) | Command-line tool for file inspection and manipulation | Building/using CLI tool |

## Architecture

Three-crate dependency graph ensures clean separation:

```
altium-derive (proc macros, no runtime deps)
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
cargo build -p altium-derive

# Run all tests
cargo test --workspace

# Build CLI binary
cargo run -p altium-cli -- --help

# Verify publishability
cargo publish --dry-run -p altium-derive
cargo publish --dry-run -p altium-format
cargo publish --dry-run -p altium-cli

# Generate documentation
cargo doc --workspace --no-deps --open
```

## CI/CD Workflows

| Workflow | What | When |
|---|---|---|
| .github/workflows/publish.yml | Publishes all three crates to crates.io | Tag push (v*.*.*) |
| .github/workflows/release.yml | Builds cross-platform binaries, creates GitHub Release | Tag push (v*.*.*) |

**Publishing workflow data flow:**
1. Tag push triggers both workflows
2. Checkout code
3. Publish derive → poll crates.io until available (5min timeout)
4. Publish format → poll crates.io until available (5min timeout)
5. Publish cli

**Release workflow data flow:**
1. Build matrix: Linux x86_64, macOS x86_64, Windows x86_64
2. `cargo build --release -p altium-cli` on each platform
3. Generate SHA256 checksums
4. Upload binaries + checksums to GitHub Release

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
altium-cli query design.SchDoc "Component[Designator=R1]"

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
