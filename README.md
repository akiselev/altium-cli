# altium-cli

Rust library and CLI tool for reading, writing, and querying Altium Designer files.

```
╔═══════════════════════════════════════════════════════════╗
║      WARNING: EXPERIMENTAL - WORK IN PROGRESS             ║
║  Breaking changes expected. Use at your own risk.         ║
║  This project depends on vibe-reverse engineering Altium  ║
║  binary files. BACKUP YOUR FILES IN VERSION CONTROL       ║
║  BEFORE USING THIS CLI. The format crate makes a best     ║
║  attempt at nondestructive editing but there will be bugs ║
╚═══════════════════════════════════════════════════════════╝
```

## Features

- **Read/Write Altium Files**: Support for `.SchLib`, `.SchDoc`, `.PcbLib`, `.PcbDoc` formats
- **Three-Layer API**: Choose your abstraction level
  - CFB-level access for reverse engineering
  - Generic dynamic API when schema varies
  - Strongly-typed API with derive macros for known record types
- **CSS-Like Query Language**: Find components, nets, and pins using intuitive selectors
- **Non-Destructive Editing**: Preserves unknown fields for safe round-trip modifications
- **Agent-Friendly CLI**: JSON output and stable schemas for scripting and automation

## Installation

### CLI Tool

Install the command-line tool via cargo:

```bash
cargo install altium-cli
```

### Library

Add the library to your Rust project:

```bash
cargo add altium-format
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
altium-format = "0.1.0"
```

## Quick Start

### Command Line

Inspect file structure:

```bash
altium-cli inspect components.SchLib
altium-cli inspect footprints.PcbLib
```

Query components using selectors:

```bash
# Find components by designator pattern
altium-cli query file.SchLib "R*"

# Find components by part number
altium-cli query file.SchLib "$LM358"

# CSS-like queries
altium-cli query file.SchLib "Component[Designator=R1]"
```

Edit schematics:

```bash
# Move component (coordinates in mils)
altium-cli edit design.SchDoc -c "move U1 1000 2000" -o output.SchDoc

# Delete component
altium-cli edit design.SchDoc -c "delete R3" -o output.SchDoc
```

Output as JSON for scripting:

```bash
altium-cli inspect library.SchLib --json | jq '.components[0].name'
```

### Library Usage

Read a schematic library:

```rust
use altium_format::io::SchLib;
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("resistors.SchLib")?;
    let lib = SchLib::open(BufReader::new(file))?;

    for component in &lib.components {
        println!("{}: {} pins", component.name, component.pin_count());
    }

    Ok(())
}
```

Query records programmatically:

```rust
use altium_format::query::query_records;
use altium_format::io::SchLib;

let lib = SchLib::open(file)?;
let resistors = query_records(&lib.records, "Component[Designator^=R]")?;

for comp in resistors {
    println!("Found resistor: {}", comp.designator);
}
```

Create footprints with the builder API:

```rust
use altium_format::footprint::FootprintBuilder;
use altium_format::records::pcb::PcbPadShape;

let mut builder = FootprintBuilder::new("SOIC-8");
builder.add_dual_row_smd(
    4,      // pads per side
    1.27,   // pitch (mm)
    5.3,    // row spacing (mm)
    1.5,    // pad width (mm)
    0.6,    // pad height (mm)
    PcbPadShape::Rectangular,
);
let component = builder.build_deterministic(&mut ());
```

## Crates

This workspace contains three crates:

- **[altium-format-derive](crates/altium-format-derive/README.md)** - Procedural macros for deriving serialization traits
- **[altium-format](crates/altium-format/README.md)** - Core library for reading and writing Altium files
- **[altium-cli](crates/altium-cli/README.md)** - Command-line tool for inspecting and editing files

See individual README files for detailed documentation.

## Building

Build all crates in the workspace:

```bash
cargo build --workspace
```

Run tests:

```bash
cargo test --workspace
```

Build the CLI binary:

```bash
cargo build --release -p altium-cli
```

The compiled binary will be at `target/release/altium-cli`.

Generate documentation:

```bash
cargo doc --workspace --no-deps --open
```

## Architecture

The library uses a trait-based design with three abstraction layers:

1. **CFB Layer**: Raw access to OLE compound document structure
2. **Generic Layer**: Dynamic parameter access without type knowledge
3. **Typed Layer**: Full deserialization with strongly-typed records

This allows you to choose the right level of abstraction for your use case, from low-level reverse engineering to high-level programmatic manipulation.

## Coordinate System

Altium uses fixed-point coordinates where **10,000 internal units = 1 mil = 0.001 inch**. All coordinate operations use the `Coord` newtype wrapper:

```rust
use altium_format::types::Coord;

let coord = Coord::from_mils(10);  // 100,000 internal units
let mm_coord = Coord::from_mm(2.54);  // 1,000,000 internal units (1 inch)
```

## Contributing

Contributions are welcome. This project uses:

- Rust 1.85+ (edition 2024)
- Property-based testing for trait contracts
- Golden file tests for format stability
- Roundtrip tests for lossless serialization

## License

GPL-3.0-only

## Author

Alexander Kiselev <alex@akiselev.com>

## Repository

<https://github.com/akiselev/altium-cli>
