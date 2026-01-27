# altium-format

Standalone library for reading/writing Altium Designer files (.SchLib, .PcbLib, .SchDoc, .PcbDoc).

## File Index

| File/Directory | What | When |
|---|---|---|
| **Core Types** | | |
| types/coord.rs | Coord type (10,000 units = 1 mil), CoordPoint, CoordRect | Reading/writing coordinates |
| types/unit.rs | Unit conversions (mil, mm, inch) | Converting between measurement systems |
| types/layer.rs | PCB layer enumeration | Working with PCB layers |
| types/color.rs | Altium color type | Rendering graphics |
| types/parameters.rs | ParameterCollection, ParameterValue | Parsing parameter-based records |
| types/mask_expansion.rs | MaskExpansion enum (Auto \| Manual(Coord)) | Pad/via mask expansion |
| **Serialization Traits** | | |
| traits/params.rs | FromParams, ToParams | Parameter-based serialization |
| traits/binary.rs | FromBinary, ToBinary | Binary record serialization |
| traits/conversion.rs | FromParamValue, ToParamValue | Individual parameter conversion |
| traits/mod.rs | SchPrimitive, PcbPrimitive, AltiumRecord | Polymorphic record access |
| **Records** | | |
| records/sch/primitive.rs | SchRecord enum, SchPrimitive trait impls | Dispatching schematic operations |
| records/sch/pin.rs | SchPin struct | Working with schematic pins |
| records/sch/component.rs | SchComponent struct | Schematic component metadata |
| records/sch/*.rs | 30 schematic record types | Type-specific operations |
| records/pcb/primitive.rs | PcbRecord enum, PcbPrimitive trait impls | Dispatching PCB operations |
| records/pcb/pad.rs | PcbPad struct | PCB pad primitives |
| records/pcb/component.rs | PcbComponent struct | PCB component metadata |
| records/pcb/*.rs | 15 PCB record types | Type-specific operations |
| **File I/O** | | |
| io/reader.rs | Block reading, decompression | Reading binary streams |
| io/writer.rs | Block writing, compression | Writing binary streams |
| io/schlib.rs | SchLib struct, open/save | SchLib file access |
| io/schdoc.rs | SchDoc struct, open/save | SchDoc file access |
| io/pcblib.rs | PcbLib struct, open/save | PcbLib file access |
| **Binary Format** | | |
| format/constants.rs | SIZE_FLAG_MASK, BLOCK_FLAG_BINARY, CFB_COMPRESSED_TAG | Magic number constants |
| format/record_ids.rs | SchRecordId enum | Schematic record type IDs |
| **API Layers** | | |
| api/cfb.rs | Layer 1: CFB (OLE compound document) access | Reverse engineering, low-level access |
| api/generic/record.rs | Layer 2: GenericRecord (dynamic access) | Schema-less queries |
| api/generic/value.rs | Dynamic Value type | Accessing unknown parameters |
| api/typed/accessor.rs | Layer 3: TypedAccessor | Strongly-typed editing |
| api/document.rs | AltiumDocument unified entry point | Opening any Altium file |
| **Operations** | | |
| ops/categorization.rs | categorize_component() shared logic | Component type detection |
| ops/queries/components.rs | components_by_category() | Grouping components |
| ops/queries/nets.rs | net_connections() | Extracting connectivity |
| ops/queries/power.rs | power_nets(), power_map() | Power net analysis |
| ops/transforms/grouping.rs | group_by_proximity() | Spatial clustering |
| ops/schdoc.rs | Schematic document operations | SchDoc queries |
| ops/schlib.rs | Schematic library operations | SchLib queries |
| ops/pcbdoc.rs | PCB document operations | PcbDoc queries |
| ops/pcblib.rs | PCB library operations | PcbLib queries |
| ops/output.rs | JSON serialization structures | CLI/API output |
| **Edit Sessions** | | |
| edit/session.rs | EditSession state machine | Non-destructive editing |
| edit/library.rs | Library editing operations | Adding/removing components |
| **Footprint Generation** | | |
| footprint/builder.rs | FootprintBuilder API | Programmatic footprint creation |
| footprint/package.rs | Package type helpers (SOIC, QFN, BGA) | Standard footprints |
| **Query Language** | | |
| query/selector.rs | CSS-like selector parsing | Component[Designator=R1] queries |
| query/pattern.rs | Pattern matching | Attribute filters |
| query/engine.rs | Query execution | Finding records |
| **Tree Structures** | | |
| tree/node.rs | RecordTree, hierarchical storage | Component/pin relationships |
| tree/walker.rs | BreadthFirstWalker | Traversing hierarchies |

## Key Patterns

| Pattern | Where | Why |
|---|---|---|
| Trait polymorphism | SchPrimitive, PcbPrimitive | Eliminate 85+ match statements |
| Three-layer API | api/ | Choose abstraction level (CFB/generic/typed) |
| State types | MaskExpansion | Type system prevents invalid states |
| Query operations | ops/queries/ | Reusable logic for CLI and programmatic use |
| Property-based testing | traits/tests.rs | Verify trait contracts across 30+ types |

## Build Commands

```bash
# Build library
cargo build -p altium-format

# Run tests
cargo test -p altium-format

# Generate docs
cargo doc -p altium-format --open
```

## Library Usage

```rust
use altium_format::io::SchLib;
use std::fs::File;
use std::io::BufReader;

// Open schematic library
let file = File::open("components.SchLib")?;
let lib = SchLib::open(BufReader::new(file))?;

// Iterate components
for component in lib.components() {
    println!("Designator: {}", component.designator);
}

// Query with CSS-like selectors
use altium_format::query::selector::Selector;
let selector = Selector::parse("Component[Designator=R1]")?;
let matches = selector.find(&lib)?;
```

For CLI usage, see [altium-cli](../altium-cli/CLAUDE.md).
