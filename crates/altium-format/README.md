# altium-format

Rust library for reading and writing Altium Designer files with typed field structs and agent-friendly CLI.

## Architecture

The library uses a V2 architecture ported from decompiled C# Altium code, with properly reverse-engineered file format structures.

```
┌─────────────────────────────────────────────────────────────┐
│                      altium-cli                              │
│  (clap binary with subcommands: inspect, query, export)      │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    altium-format                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   ops/      │  │   query/    │  │   edit/     │          │
│  │ (queries)   │  │ (selectors) │  │ (sessions)  │          │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘          │
│         │                │                │                  │
│  ┌──────▼────────────────▼────────────────▼──────┐          │
│  │                   v2/                          │          │
│  │  ┌─────────────────────────────────────────┐  │          │
│  │  │ v2/io/      SchLibV2, SchDocV2          │  │          │
│  │  │             PcbLibV2, PcbDocV2          │  │          │
│  │  └──────────────────┬──────────────────────┘  │          │
│  │                     │                          │          │
│  │  ┌──────────────────▼──────────────────────┐  │          │
│  │  │ v2/serializer/  ASCII + Binary modes    │  │          │
│  │  │                 export_*/import_*       │  │          │
│  │  └──────────────────┬──────────────────────┘  │          │
│  │                     │                          │          │
│  │  ┌──────────────────▼──────────────────────┐  │          │
│  │  │ v2/fields/      PinData, ComponentData  │  │          │
│  │  │                 TypedRecord enum        │  │          │
│  │  └─────────────────────────────────────────┘  │          │
│  └────────────────────────────────────────────────┘          │
│                                                              │
│  ┌───────────────────────────────────────────────┐          │
│  │                  types/                        │          │
│  │  Coord, Color, Layer, ParameterCollection     │          │
│  └───────────────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

### Reading Files

```
CFB file (.SchLib, .PcbLib, .SchDoc, .PcbDoc)
         │
         ▼
    v2/io/schlib.rs (or pcblib.rs, schdoc.rs, pcbdoc.rs)
         │  - Opens CFB compound document
         │  - Reads section keys for component paths
         │  - Parses Data stream (ASCII or Binary mode)
         ▼
    v2/serializer/format_v5/import_*
         │  - Deserializes record fields
         │  - Handles ASCII: |KEY=VALUE| params
         │  - Handles Binary: sequential typed fields
         ▼
    v2/fields/ (PinData, ComponentData, etc.)
         │  - Typed field structs with proper scales
         │  - Ready for ops/ queries
         ▼
    ops/ queries and transforms
         │
         ▼
    CLI JSON output or API access
```

### Writing Files

```
PinData, ComponentData, etc.
         │
         ▼
    v2/serializer/format_v5/export_*
         │  - Serializes to ASCII or Binary
         │  - Handles extended data (PinFrac, PinWideText)
         ▼
    v2/io/schlib.rs (or pcblib.rs, etc.)
         │  - Builds CFB section structure
         │  - Generates section keys (30 char limit)
         │  - Writes Data and extended streams
         ▼
    CFB file output
```

## Coordinate Systems

Altium uses two distinct coordinate scales. Using the wrong scale causes silent precision bugs.

### Schematic Coordinates (V2Coord)

- **Scale**: 100,000 internal units per mil
- **Source**: Confirmed from decompiled `SchDataSerializerBinary.Export_Coord`
- **Binary format**: Whole-mil `i16` in Data stream + fractional `i32` in PinFrac stream
- **Usage**: All schematic files (.SchLib, .SchDoc)

```rust
use altium_format::v2::V2Coord;

let coord = V2Coord::from_mils(10.5);
assert_eq!(coord.to_raw(), 1_050_000);  // 10.5 * 100,000

// Binary split for storage
let (whole, frac) = coord.to_binary_parts();
assert_eq!(whole, 10);      // Stored in Data stream
assert_eq!(frac, 50_000);   // Stored in PinFrac stream
```

### PCB Coordinates (PcbCoord)

- **Scale**: 10,000 internal units per mil
- **Source**: Confirmed from Altium SDK (`k1Mil = 10000`)
- **Binary format**: Direct `i32` in binary records
- **Usage**: All PCB files (.PcbLib, .PcbDoc)

```rust
use altium_format::v2::pcb::PcbCoord;

let coord = PcbCoord::from_mils(10.5);
assert_eq!(coord.to_raw(), 105_000);  // 10.5 * 10,000
```

## Type Hierarchy

### TypedRecord Enum

Runtime dispatch for parsed schematic records:

```rust
use altium_format::v2::{TypedRecord, PinData, ComponentData};

match record {
    TypedRecord::Pin(pin) => println!("Pin: {}", pin.name),
    TypedRecord::Component(comp) => println!("Component: {}", comp.lib_ref),
    TypedRecord::Wire(wire) => println!("Wire from {:?}", wire.locations),
    TypedRecord::Unknown(id) => println!("Unknown record type: {}", id),
    // ... 30+ variants
}
```

### Base Structs

Shared fields across record types:

- **DataObjectBase**: `owner_index`, `is_not_accessible`, `index_in_sheet`
- **GraphicalObjectBase**: extends DataObjectBase + `owner_part_id`, `owner_part_display_mode`
- **RectangularEntryContainerBase**: location, size, colors for sheet symbols
- **BasicEntryObjectBase**: side, distance, colors for sheet entries

## Serialization Flow

### SchSerializer Trait

Abstracts ASCII vs Binary encoding:

```rust
pub trait SchSerializer {
    fn export_coord(&mut self, value: i32, name: &str) -> Result<()>;
    fn import_coord(&mut self, name: &str) -> Result<i32>;
    fn export_string(&mut self, value: &str, name: &str) -> Result<()>;
    fn import_string(&mut self, name: &str) -> Result<String>;
    // ... 50+ methods for different types
}
```

### ASCII Mode (Mode 0)

- Format: `|RECORD=2|LOCATION.X=100|NAME=VCC|...`
- Coordinates as mil strings: `LOCATION.X=100` means 100 mils
- Used in older files and some stream types

### Binary Mode (Mode 1)

- Sequential typed fields, no delimiters
- Coordinates split: whole mils as `i16`, fractional in extended streams
- Extended data streams: `PinFrac`, `PinWideText`, `PinTextData`, `PinSymbolLineWidth`

## Key Invariants

1. **PCB coordinates always use 10K units/mil** (PcbCoord type)
2. **Sch coordinates always use 100K units/mil** (V2Coord type)
3. **Section keys are max 30 chars** with collision avoidance suffix
4. **Extended data streams required** for pins with fractional coordinates or wide text
5. **OWNERINDEX consistency**: Parent-child relationships encoded via owner_index field

## CFB Storage Structure

Altium files are COM/OLE Compound Storage (CFB) containers:

```
SchLib file:
├── FileHeader              # Library metadata, component list
├── SectionKeys             # Maps LIBREF to section path
├── ComponentA/             # One folder per component
│   ├── Data                # ASCII or Binary record stream
│   ├── PinFrac             # Fractional coordinate parts
│   ├── PinWideText         # Unicode text overflow
│   ├── PinTextData         # Additional text data
│   └── PinSymbolLineWidth  # Symbol line widths
└── ComponentB/
    └── ...
```

## Usage Examples

### Reading a SchLib

```rust
use altium_format::v2::io::schlib::SchLibV2;

let lib = SchLibV2::open_file("components.SchLib")?;

for comp in &lib.components {
    println!("Component: {}", comp.entry.lib_ref);

    // Typed pin access
    for pin in comp.pins() {
        println!("  Pin {}: {} ({:?})",
            pin.designator,
            pin.name,
            pin.electrical_type
        );
    }
}
```

### Reading a PcbLib

```rust
use altium_format::v2::pcb::io::pcblib::PcbLibV2;

let lib = PcbLibV2::open_file("footprints.PcbLib")?;

for footprint in &lib.footprints {
    println!("Footprint: {}", footprint.pattern_name);

    for pad in &footprint.pads {
        println!("  Pad {}: {:?} at ({}, {})",
            pad.name,
            pad.shape,
            pad.x.to_mils(),
            pad.y.to_mils()
        );
    }
}
```

### Querying Components

```rust
use altium_format::query::query_records;
use altium_format::v2::io::schlib::SchLibV2;

let lib = SchLibV2::open_file("library.SchLib")?;
let resistors = query_records(&lib, "Component[Designator^=R]")?;

for comp in resistors {
    println!("Found resistor: {}", comp.entry.lib_ref);
}
```

## Testing Strategy

1. **Property-based tests**: Roundtrip (export then import = identity) for all record types
2. **Real file integration**: Test with actual Altium libraries
3. **CFB roundtrip tests**: `v2_schlib_cfb_roundtrip.rs`, `v2_pcblib_cfb_roundtrip.rs`
4. **Golden file tests**: Lock CLI output format for stability

## Build Instructions

```bash
# Build library
cargo build -p altium-format

# Run tests
cargo test -p altium-format

# Generate docs
cargo doc -p altium-format --open
```

## License

GPL-3.0-only

## Repository

<https://github.com/akiselev/altium-cli>
