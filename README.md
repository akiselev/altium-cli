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

- **Read/Write Altium Files**: Support for `.SchLib`, `.SchDoc`, `.PcbLib`, `.PcbDoc`, `.PrjPcb`, `.IntLib`
- **Comprehensive CLI**: 100+ commands for analysis, editing, and project management
- **CSS-Like Query Language**: Find components, nets, and pins using intuitive selectors
- **Non-Destructive Editing**: Preserves unknown fields for safe round-trip modifications
- **Agent-Friendly**: JSON output and stable schemas for scripting and AI automation

## Installation

### CLI Tool

```bash
cargo install altium-cli
```

### Library

```bash
cargo add altium-format
```

## Quick Start

### Inspect Files

```bash
altium-cli inspect components.SchLib
altium-cli inspect design.PcbDoc --json
```

### Query Components

```bash
# Find by designator pattern
altium-cli query design.SchDoc "R*"

# Find by part number
altium-cli query design.SchDoc "$LM358"

# CSS-like queries
altium-cli query design.SchDoc "component[part*=MCU]"
```

### Schematic Analysis

```bash
altium-cli schdoc overview design.SchDoc
altium-cli schdoc bom design.SchDoc
altium-cli schdoc netlist design.SchDoc
altium-cli schdoc power-map design.SchDoc
altium-cli schdoc components design.SchDoc
```

### PCB Analysis

```bash
altium-cli pcbdoc overview design.PcbDoc
altium-cli pcbdoc rules design.PcbDoc
altium-cli pcbdoc components design.PcbDoc --layer top
altium-cli pcbdoc nets design.PcbDoc
altium-cli pcbdoc layers design.PcbDoc
```

### Project Management

```bash
altium-cli prjpcb overview project.PrjPcb
altium-cli prjpcb bom project.PrjPcb --grouped
altium-cli prjpcb netlist project.PrjPcb
altium-cli prjpcb validate project.PrjPcb --check-files
altium-cli prjpcb diff-sch-pcb project.PrjPcb
```

### Library Browsing

```bash
# Schematic libraries
altium-cli schlib list components.SchLib
altium-cli schlib search components.SchLib "op amp"
altium-cli schlib component components.SchLib LM358

# PCB libraries
altium-cli pcblib list footprints.PcbLib
altium-cli pcblib measure footprints.PcbLib SOIC-8
altium-cli pcblib render-ascii footprints.PcbLib SOIC-8

# Integrated libraries
altium-cli intlib list library.IntLib
altium-cli intlib search library.IntLib "LM358"
altium-cli intlib extract-schlib library.IntLib -o symbols.SchLib
```

### Editing

```bash
# Schematic editing
altium-cli edit design.SchDoc -c "move U1 1000 2000"
altium-cli edit design.SchDoc -c "delete R3"
altium-cli edit design.SchDoc -c "add-wire 100,100,200,200"

# Library creation
altium-cli schlib create new.SchLib
altium-cli schlib add-component lib.SchLib MyPart
altium-cli schlib gen-ic lib.SchLib IC1 --pins "VCC,GND,IN,OUT"

altium-cli pcblib create new.PcbLib
altium-cli pcblib gen-chip lib.PcbLib 0603
altium-cli pcblib add-dual-row lib.PcbLib SOIC-8 8 --pitch 50 --span 300

# PCB editing
altium-cli pcbdoc add-track design.PcbDoc 0 0 100 100 --net VCC
altium-cli pcbdoc add-via design.PcbDoc 500 500 --net VCC
altium-cli pcbdoc add-rule design.PcbDoc clearance --value 10
```

## Command Groups

| Group     | Purpose                     | Example                                        |
| --------- | --------------------------- | ---------------------------------------------- |
| `inspect` | Quick file overview         | `altium-cli inspect file.SchLib`               |
| `query`   | Find records with selectors | `altium-cli query file.SchDoc "R*"`            |
| `edit`    | Modify schematics           | `altium-cli edit file.SchDoc -c "move U1 0 0"` |
| `schdoc`  | Schematic document analysis | `altium-cli schdoc bom design.SchDoc`          |
| `schlib`  | Schematic library ops       | `altium-cli schlib list lib.SchLib`            |
| `pcbdoc`  | PCB document analysis/edit  | `altium-cli pcbdoc rules design.PcbDoc`        |
| `pcblib`  | PCB library ops             | `altium-cli pcblib measure lib.PcbLib FP`      |
| `prjpcb`  | Project management          | `altium-cli prjpcb bom project.PrjPcb`         |
| `intlib`  | Integrated library access   | `altium-cli intlib search lib.IntLib "LM"`     |

See [docs/](docs/) for detailed command reference.

## Library Usage

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

## Crates

| Crate                                                | Purpose                             |
| ---------------------------------------------------- | ----------------------------------- |
| [altium-format-derive](crates/altium-format-derive/) | Procedural macros for serialization |
| [altium-format](crates/altium-format/)               | Core library for Altium file I/O    |
| [altium-cli](crates/altium-cli/)                     | Command-line tool                   |

## Supported File Types

| Extension | Type               | Read | Write | Query |
| --------- | ------------------ | ---- | ----- | ----- |
| `.SchLib` | Schematic Library  | Yes  | Yes   | Yes   |
| `.SchDoc` | Schematic Document | Yes  | Yes   | Yes   |
| `.PcbLib` | PCB Library        | Yes  | Yes   | Yes   |
| `.PcbDoc` | PCB Document       | Yes  | Yes   | Yes   |
| `.PrjPcb` | PCB Project        | Yes  | Yes   | -     |
| `.IntLib` | Integrated Library | Yes  | -     | Yes   |

## Coordinate System

Altium uses fixed-point coordinates: **10,000 internal units = 1 mil = 0.001 inch**

All CLI coordinates are in mils.

## Building

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build release binary
cargo build --release -p altium-cli
```

## License

GPL-3.0-only

## Author

Alexander Kiselev <alex@akiselev.com>

## Repository

<https://github.com/akiselev/altium-cli>
