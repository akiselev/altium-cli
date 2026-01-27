# altium-format

Rust library for reading and writing Altium Designer files with a trait-based API and agent-friendly CLI.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      altium-cli                              │
│  (clap binary with subcommands: inspect, query, export)      │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    altium-format                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   api/      │  │   ops/      │  │   edit/     │          │
│  │ (3-layer)   │  │ (queries)   │  │ (sessions)  │          │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘          │
│         │                │                │                  │
│  ┌──────▼────────────────▼────────────────▼──────┐          │
│  │                  records/                      │          │
│  │  SchPrimitive trait ◄── SchRecord variants    │          │
│  │  PcbPrimitive trait ◄── PcbRecord variants    │          │
│  └──────────────────────┬────────────────────────┘          │
│                         │                                    │
│  ┌──────────────────────▼────────────────────────┐          │
│  │                   io/                          │          │
│  │  reader.rs, writer.rs, schdoc.rs, pcbdoc.rs   │          │
│  └──────────────────────┬────────────────────────┘          │
│                         │                                    │
│  ┌──────────────────────▼────────────────────────┐          │
│  │                  types/                        │          │
│  │  Coord, Color, Layer, ParameterCollection     │          │
│  └───────────────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

```
Altium File (.SchLib, .PcbDoc, etc.)
         │
         ▼
    cfb crate (OLE compound document)
         │
         ▼
    io/reader.rs (block parsing, decompression)
         │
         ▼
    api/generic/ (ParameterCollection, Value)
         │
         ▼
    FromParams/FromBinary traits
         │
         ▼
    Typed records (SchComponent, PcbPad, etc.)
         │
         ▼
    SchPrimitive/PcbPrimitive trait access
         │
         ▼
    ops/ queries and transforms
         │
         ▼
    CLI JSON output
```

## Core Abstractions

### Three-Layer API

Users choose their abstraction level:

1. **Layer 1 (CFB)**: Raw access to compound file structure
   - Use when: Reverse engineering unknown file types
   - Example: `doc.cfb().streams()` lists all internal files

2. **Layer 2 (Generic)**: Dynamic access without type knowledge
   - Use when: Schema is unknown or varies
   - Example: `record.get("LIBREFERENCE")` accesses any parameter

3. **Layer 3 (Typed)**: Full deserialization with derive macros
   - Use when: Working with known record types
   - Example: `component.lib_reference` provides type-safe access

### Trait Hierarchy

**SchPrimitive** and **PcbPrimitive** traits enable polymorphic dispatch:

- `RECORD_ID` / `OBJECT_ID`: Type identifier constant
- `owner_index()` / `set_owner_index()`: Hierarchy management
- `location()`: Optional coordinate access
- `record_type_name()`: Diagnostic type name
- `get_property()`: Dynamic property lookup
- `calculate_bounds()`: Bounding rectangle

Implemented by 30 schematic types and 15 PCB types. Adding a new record type requires only implementing the trait, not updating scattered match statements.

### State Types Over Boolean Pairs

**MaskExpansion** replaces implicit state machines:

```rust
// Before: manual + value creates invalid states
pub paste_mask_expansion_manual: bool,
pub paste_mask_expansion: Coord,

// After: type system prevents invalid states
pub paste_mask_expansion: MaskExpansion,  // Auto | Manual(Coord)
```

Validation happens at construction, not at use. Corrupted data from files returns `Result::Err` rather than panicking for recoverability.

## Coordinate System

Altium uses fixed-point coordinates:

- **10,000 internal units = 1 mil = 0.001 inch**
- All coordinate operations use `Coord` newtype wrapper
- `Coord::from_mils(10)` = 100,000 internal units
- `Coord::from_mm(2.54)` = 1,000,000 internal units (1 inch)

Integer storage avoids floating-point precision issues. Extract raw value with `.to_raw()` for arithmetic.

## Roundtrip Fidelity Invariants

1. **FromParams + ToParams must be inverses**
   - Property-based tests verify: `from_params(to_params(x)) == x`
   - Unknown fields preserved in `UnknownFields`

2. **FromBinary + ToBinary must be inverses**
   - Binary format must match byte-for-byte
   - Golden file tests lock format compatibility

3. **OWNERINDEX consistency**
   - Parent-child relationships encoded via OWNERINDEX
   - Tree operations preserve hierarchy integrity
   - Editing sessions track index mutations

## Operations Design

**ops/** modules provide pure query functions that return data, not formatted output:

- `components_by_category()` returns `HashMap<Category, Vec<Component>>`
- `net_connections()` returns `Vec<NetConnection>`
- `power_map()` returns `HashMap<NetName, Count>`

CLI commands compose these operations. Same logic powers programmatic API and command-line tools.

## Deterministic UUID Context

Standalone library uses `uuid::Uuid::new_v4()` for standard random UUIDs. Cadatomic fork replaces `()` parameter with `DeterminismContext` for event sourcing replay.

Signature: `build_deterministic(&mut ())` accepts unit type; fork swaps in context type for deterministic generation.

## Format Quirks

### Display Mode Bounds

`display_mode` indexes into array of `display_mode_count` elements. Out-of-bounds access causes undefined behavior when rendering symbol views.

Validation: `set_display_mode()` returns `Result::Err` for invalid indices rather than panicking. Corrupted files should be recoverable.

### Manual Flag Values

Altium binary format uses flag value `2` (not `1`) for manual mode, `0` for auto mode. Observed from PCB1.PcbLib test files. `MaskExpansion` implementation uses `!= 0` check for robustness against format variations.

### Parameter vs Binary Records

- **Schematic records**: Parameter-based (key=value pairs)
  - Format: `|RECORD=1|NAME=Test|X=100mil|`
  - Trait: `FromParams`, `ToParams`

- **PCB records**: Binary-based (fixed structs)
  - Format: Block header + packed binary data
  - Trait: `FromBinary`, `ToBinary`

Different file types require different deserialization paths.

## CLI Agent-Friendly Design

**JSON-first output** for programmatic consumption:

```bash
altium-cli inspect library.SchLib | jq '.components[0].name'
```

**Selector queries** for filtering:

```bash
altium-cli query library.SchLib "Component[Designator^=U]"  # ICs only
```

**Stable schemas** locked by golden file tests. Breaking changes increment semver.

## Build Instructions

### Library

```bash
cargo build -p altium-format
cargo test -p altium-format
cargo doc -p altium-format --open
```

### CLI Binary

```bash
cargo build --bin altium-cli
cargo install --path crates/altium-format
altium-cli --help
```

### Shell Completions

```bash
altium-cli completions bash > /usr/share/bash-completion/completions/altium-cli
```

## Usage Examples

### Reading a Library

```rust
use altium_format::io::SchLib;
use std::fs::File;
use std::io::BufReader;

let file = File::open("resistors.SchLib")?;
let lib = SchLib::open(BufReader::new(file))?;

for component in &lib.components {
    println!("{}: {} pins", component.name, component.pin_count());
}
```

### Creating a Footprint

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

### Querying Components

```rust
use altium_format::query::query_records;
use altium_format::io::SchLib;

let lib = SchLib::open(file)?;
let resistors = query_records(&lib.records, "Component[Designator^=R]")?;

for comp in resistors {
    println!("Found resistor: {}", comp.designator);
}
```

### Editing Sessions

```rust
use altium_format::edit::EditSession;

let mut session = EditSession::new(lib);
session.set_component_property("R1", "VALUE", "10k");
session.commit()?;
session.save("modified.SchLib")?;
```

## Testing Strategy

1. **Property-based tests**: Verify trait contracts across all record types
2. **Real file integration**: Test with actual Altium libraries
3. **Golden file tests**: Lock CLI output format for stability
4. **Roundtrip tests**: Ensure serialization is lossless

## License

GPL-3.0-only

## Repository

<https://github.com/akiselev/altium-cli>
