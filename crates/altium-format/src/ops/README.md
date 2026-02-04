# ops/ - High-level Operations Module

This module provides reusable business logic for Altium file operations. It serves as the middle layer between CLI commands and low-level file I/O.

## Architecture

```
altium-cli (binary)
    |
    v
commands/schlib.rs  <-- CLI parsing (clap)
    |
    v
ops/schlib.rs       <-- Business logic (cmd_* functions)
    |
    v
v2/io/schlib.rs     <-- File I/O (SchLibV2::open/write)
    |
    v
v2/fields/*.rs      <-- Typed records (PinData, ComponentData, etc.)
```

## Data Flow

```
CLI Input --> Parse Args --> Open File --> Extract Data --> Format Output
                                |
                                v
                         SchLibV2 struct
                                |
                          +-----+-----+
                          |           |
                     components    header
                          |
                     TypedRecord[]
                          |
                    +-----+-----+
                    |     |     |
                  Pin  Line  Rect  ...
```

## Why This Structure

- **ops/ layer**: Separates business logic from CLI parsing. Enables library API usage without CLI dependency.
- **commands/ layer**: Thin wrappers that handle clap types and output formatting. Contains no business logic.
- **v2/ layer**: Low-level I/O that owns CFB parsing and record serialization. Provides the foundation for ops functions.

This structure matches the pcblib/pcbdoc pattern. Consistency across modules aids maintenance and reduces cognitive load.

## Invariants

- Each file type appears in exactly one ops module (no cross-module file access)
- `cmd_*` functions return `Result<OutputType, Box<dyn Error>>` - consistent error handling
- Output types are in `ops/output.rs` - single source for all serializable results
- CLI commands map 1:1 to ops functions - no CLI-only logic

## Module Overview

| Module | File Type | Description |
|--------|-----------|-------------|
| `schlib.rs` | `.SchLib` | Schematic library browse, search, create, edit |
| `schdoc.rs` | `.SchDoc` | Schematic document analysis, BOM, netlist |
| `pcblib.rs` | `.PcbLib` | PCB library browse, measure, create, edit |
| `pcbdoc.rs` | `.PcbDoc` | PCB document analysis, rules, routing |
| `prjpcb.rs` | `.PrjPcb` | Project overview, BOM, validation |
| `intlib.rs` | `.IntLib` | Integrated library browse, extract |
| `output.rs` | - | Serializable output data structures |
| `categorization.rs` | - | Shared component categorization logic |

## Usage Example

```rust
use altium_format::ops::schlib;
use std::path::Path;

// Get library overview
let overview = schlib::cmd_overview(Path::new("components.SchLib"))?;
println!("Components: {}", overview.component_count);

// Search for components
let results = schlib::cmd_search(Path::new("components.SchLib"), "LM358", None)?;
for comp in results.results {
    println!("Found: {}", comp.name);
}
```
