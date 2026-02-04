# altium-format

Standalone library for reading/writing Altium Designer files (.SchLib, .PcbLib, .SchDoc, .PcbDoc).

## File Index

| File/Directory | What | When |
|---|---|---|
| **V2 Core (Primary API)** | | |
| v2/mod.rs | V2 module root, re-exports PinData, ComponentData, TypedRecord | Accessing v2 types |
| v2/coord.rs | V2Coord (100K units/mil), V2Point | Schematic coordinates |
| v2/types.rs | ObjectId, PinElectrical, RotationBy90, enums | Type discriminants |
| v2/consts.rs | Parameter name constants from FileFormatConsts.cs | Serialization keys |
| **V2 Field Structs** | | |
| v2/fields/mod.rs | TypedRecord enum, DataObjectBase, GraphicalObjectBase | Runtime dispatch |
| v2/fields/pin.rs | PinData struct | Schematic pin access |
| v2/fields/component.rs | ComponentData struct | Component metadata |
| v2/fields/parameter.rs | ParameterData struct | Component parameters |
| v2/fields/primitives.rs | ArcData, LineData, RectangleData, etc. | Drawing primitives |
| v2/fields/schematic.rs | WireData, BusData, JunctionData, NetLabelData, etc. | Connectivity |
| v2/fields/sheet.rs | SheetData, SheetSymbolData, SheetEntryData | Hierarchy |
| v2/fields/implementation.rs | ImplementationData, ImplementationListData | Footprint links |
| **V2 Serializer** | | |
| v2/serializer/mod.rs | SchSerializer trait (export/import methods) | Encoding interface |
| v2/serializer/ascii.rs | AsciiSerializer: `\|KEY=VALUE\|` format | Mode 0 encoding |
| v2/serializer/binary.rs | BinarySerializer: sequential typed fields | Mode 1 encoding |
| v2/serializer/format_v5/mod.rs | export_pin, import_pin, export_component, etc. | Record serialization |
| **V2 Schematic I/O** | | |
| v2/io/mod.rs | SchLib/SchDoc I/O module | File access |
| v2/io/schlib.rs | SchLibV2::open(), SchLibV2::write() | SchLib files |
| v2/io/schdoc.rs | SchDocV2::open(), SchDocV2::write() | SchDoc files |
| v2/io/section_keys.rs | Section key generation (30 char limit) | CFB storage paths |
| **V2 PCB** | | |
| v2/pcb/mod.rs | PCB module root, re-exports PcbCoord, PcbObjectId | PCB types |
| v2/pcb/coord.rs | PcbCoord (10K units/mil), PcbPoint | PCB coordinates |
| v2/pcb/enums.rs | PcbObjectId, PcbPadShape, PcbLayer, etc. | PCB discriminants |
| v2/pcb/pad.rs | PcbPadV2 struct | PCB pad records |
| v2/pcb/track.rs | PcbTrackV2 struct | PCB track records |
| v2/pcb/via.rs | PcbViaV2 struct | PCB via records |
| v2/pcb/component.rs | PcbComponentV2 struct | PCB component records |
| v2/pcb/io/pcblib.rs | PcbLibV2::open(), PcbLibV2::write() | PcbLib files |
| v2/pcb/io/pcbdoc.rs | PcbDocV2::open(), PcbDocV2::write() | PcbDoc files |
| v2/pcb/io/streams.rs | Binary stream reading/writing | PCB binary format |
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
| **Legacy Types (types/)** | | |
| types/coord.rs | Coord type (shared utilities) | Unit conversions |
| types/unit.rs | Unit conversions (mil, mm, inch) | Measurement systems |
| types/layer.rs | PCB layer enumeration | Layer access |
| types/color.rs | Altium color type | Rendering |
| types/parameters.rs | ParameterCollection, ParameterValue | Parameter parsing |
| **Edit Sessions** | | |
| edit/session.rs | EditSession state machine | Non-destructive editing |
| edit/library.rs | Library editing operations | Adding/removing components |
| **Query Language** | | |
| query/selector.rs | CSS-like selector parsing | Component queries |
| query/pattern.rs | Pattern matching | Attribute filters |
| query/engine.rs | Query execution | Finding records |
| **Tree Structures** | | |
| tree/node.rs | RecordTree, hierarchical storage | Component/pin relationships |
| tree/walker.rs | BreadthFirstWalker | Traversing hierarchies |

## Key Patterns

| Pattern | Where | Why |
|---|---|---|
| Typed field structs | v2/fields/ | Direct field access without enum dispatch |
| Separate coordinate types | V2Coord, PcbCoord | Different scales prevent silent precision bugs |
| Serializer trait | v2/serializer/ | ASCII/Binary encoding abstracted from data |
| Query operations | ops/queries/ | Reusable logic for CLI and programmatic use |

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
use altium_format::v2::io::schlib::SchLibV2;
use altium_format::v2::{PinData, ComponentData};

// Open schematic library
let lib = SchLibV2::open_file("components.SchLib")?;

// Iterate components with typed access
for comp in &lib.components {
    println!("Component: {}", comp.entry.lib_ref);
    for pin in comp.pins() {
        println!("  Pin: {} ({})", pin.name, pin.designator);
    }
}
```

For CLI usage, see [altium-cli](../altium-cli/CLAUDE.md).
