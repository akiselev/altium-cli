# Clean-Sheet Design: altium-format

## The Insight

V2's `format_v5` already has **58+ complete export/import function pairs** covering every record type. V2 has the correct coordinate system (100K for schematic, 10K for PCB), correct field names, correct binary layout, and proven roundtrip tests. V1's record types, derive macros, and traits are entirely redundant for format handling.

What v1 has that v2 lacks is **infrastructure on top of format handling**: query engine, ops layer, record hierarchy, edit sessions. These don't depend on v1's format code — they depend on having typed records and a tree structure. We can give them that using v2's types directly.

**Decision:** V2 becomes the crate. Everything else is removed or rebuilt on top of it. The `altium-format-derive` crate is eliminated.

## What We Keep

| Source | What | Why |
|--------|------|-----|
| v2/serializer/ | SchSerializer trait, AsciiSerializer, BinarySerializer | Proven, covers all serialization complexity |
| v2/serializer/format_v5/ | 58+ export/import functions | Complete record coverage, tested |
| v2/fields/ | PinData, ComponentData, all field structs | Correct field names, correct types |
| v2/types.rs | 21+ enums (ObjectId, PinElectrical, etc.) | Complete mappings from C# |
| v2/consts.rs | 290+ parameter constants | Ground truth field names |
| v2/coord.rs | V2Coord (100K), coordinate split logic | Correct for schematics |
| v2/pcb/ | PCB binary format, PcbCoord (10K) | Correct for PCB |
| v2/io/ | SchLib, SchDoc, PcbLib, PcbDoc I/O | Proven roundtrip |
| v1 query/ | SelectorEngine, SchQL, selectors, patterns | Production-ready, 57+ tests |
| v1 tree/ | RecordTree, BreadthFirstWalker | Hierarchical record access |
| v1 ops/ | Business logic (BOM, netlist, component queries) | CLI depends on this |
| v1 footprint/ | FootprintBuilder, package helpers | Works, PCB-only, no changes needed |

## What We Remove

| Source | What | Why |
|--------|------|-----|
| altium-format-derive | Entire crate | V2 has manual impls for all record types |
| records/sch/*.rs | 30+ v1 schematic record structs | Redundant — v2 fields/ covers everything |
| records/pcb/*.rs | 15+ v1 PCB record structs | Redundant — v2 pcb/ covers everything |
| traits/params.rs | FromParams, ToParams | Replaced by SchSerializer |
| traits/binary.rs | FromBinary, ToBinary | Replaced by v2 PCB read/write |
| traits/conversion.rs | FromParamValue, ToParamValue | Replaced by v2 types |
| types/coord.rs | Coord (10K — WRONG for schematics) | Replaced by SchCoord + PcbCoord |
| types/parameters.rs | ParameterCollection | Replaced by v2 AsciiSerializer |
| io/schlib.rs | v1 SchLib | Replaced by v2 SchLibV2 |
| io/schdoc.rs | v1 SchDoc | Replaced by v2 SchDocV2 |
| io/pcblib.rs | v1 PcbLib | Replaced by v2 PcbLibV2 |
| io/pcbdoc.rs | v1 PcbDoc | Replaced by v2 PcbDocV2 |
| api/ | AltiumDocument, GenericRecord, TypedAccessor | Superseded by Document |

## Target Module Structure

```
crates/altium-format/src/
├── lib.rs
├── error.rs
│
├── format/                         # Format knowledge (v2 code, promoted)
│   ├── mod.rs
│   ├── coord.rs                    # SchCoord(100K), PcbCoord(10K)
│   ├── types.rs                    # ObjectId, PinElectrical, etc.
│   ├── consts.rs                   # Parameter name constants
│   ├── serializer/
│   │   ├── mod.rs                  # SchSerializer trait
│   │   ├── ascii.rs                # AsciiSerializer
│   │   └── binary.rs              # BinarySerializer
│   └── records/                    # export/import functions (was format_v5)
│       ├── mod.rs                  # Base helpers (export_data_object, etc.)
│       ├── pin.rs                  # export_pin, import_pin
│       ├── component.rs            # export_component, import_component
│       ├── primitives.rs           # Arc, Line, Rectangle, etc.
│       ├── schematic.rs            # Wire, Bus, Junction, etc.
│       ├── parameter.rs            # Parameter, Designator, etc.
│       ├── sheet.rs                # Sheet, Library, Font, etc.
│       ├── block.rs                # FunctionalBlock, SchematicBlock, etc.
│       ├── harness.rs              # Harness types
│       ├── implementation.rs       # Implementation, ImplementationList
│       └── misc.rs                 # ErrorMarker, Blanket, etc.
│
├── sch/                            # Schematic types (thin wrappers)
│   ├── mod.rs
│   ├── record.rs                   # SchRecord enum (dispatch over all types)
│   ├── primitive.rs                # SchPrimitive trait
│   ├── pin.rs                      # pub type SchPin = format::fields::PinData (or newtype)
│   ├── component.rs                # pub type SchComponent = ComponentData
│   └── ...                         # One per record type
│
├── pcb/                            # PCB types (from v2::pcb, promoted)
│   ├── mod.rs
│   ├── record.rs                   # PcbRecord enum
│   ├── primitive.rs                # PcbPrimitive trait
│   ├── pad.rs, track.rs, via.rs, ...
│   └── io/                         # PCB binary I/O
│       ├── pcbdoc.rs
│       ├── pcblib.rs
│       └── streams.rs
│
├── io/                             # File I/O (from v2::io, promoted)
│   ├── mod.rs
│   ├── cfb.rs                      # CFB utilities
│   ├── schlib.rs                   # SchLib open/save
│   ├── schdoc.rs                   # SchDoc open/save
│   └── section_keys.rs             # Section key mapping (30-char, collision-avoidance)
│
├── tree/                           # Record hierarchy (from v1, adapted)
│   ├── node.rs                     # RecordTree<SchRecord>
│   └── walker.rs                   # BreadthFirstWalker
│
├── query/                          # Query engine (from v1, adapted to new types)
│   ├── mod.rs
│   ├── selector.rs                 # CSS-like selector parsing
│   ├── engine.rs                   # SelectorEngine
│   ├── pattern.rs                  # Pattern matching
│   ├── view.rs                     # ComponentView, PinView, etc.
│   └── executor.rs                 # SchQL executor
│
├── ops/                            # Operations (from v1, adapted to new types)
│   ├── mod.rs
│   ├── output.rs                   # JSON serialization
│   ├── schlib.rs                   # SchLib operations
│   ├── schdoc.rs                   # SchDoc operations
│   ├── pcblib.rs                   # PcbLib operations
│   ├── pcbdoc.rs                   # PcbDoc operations
│   └── queries/                    # Reusable query operations
│       ├── components.rs
│       ├── nets.rs
│       └── power.rs
│
├── edit/                           # Editing (from v1, adapted)
│   ├── session.rs
│   └── library.rs
│
└── footprint/                      # Footprint builder (from v1, unchanged)
    ├── builder.rs
    └── package.rs
```

Note: the `format/fields/` directory from v2 is absorbed into `format/records/` — the field structs live alongside their export/import functions.

## Type Design

### Schematic Records

V2's field structs (`PinData`, `ComponentData`, etc.) become the canonical types. The `sch/` module provides:

1. **Type aliases or thin newtypes** for ergonomic names:
```rust
// sch/pin.rs
pub use crate::format::records::PinData as SchPin;
// or newtype if we need to add methods:
pub struct SchPin(pub(crate) PinData);
```

2. **SchRecord enum** for dispatch:
```rust
pub enum SchRecord {
    Pin(SchPin),
    Component(SchComponent),
    Wire(WireData),
    Bus(BusData),
    // ... all 58+ record types
}
```

3. **SchPrimitive trait** for polymorphic access:
```rust
pub trait SchPrimitive {
    fn owner_index(&self) -> i32;
    fn record_type(&self) -> ObjectId;
    fn location(&self) -> Option<(SchCoord, SchCoord)>;
    fn get_property(&self, name: &str) -> Option<String>;
}
```

Every field struct implements `SchPrimitive` directly (no derive macros — manual impls, one per type, using the v2 field accessors that already exist).

### Coordinates

```rust
// format/coord.rs
pub struct SchCoord(pub i32);  // 100,000 units per mil
pub struct PcbCoord(pub i32);  // 10,000 units per mil
```

No `Coord` generic. No `CoordScale` enum. Two types, two domains, compiler enforces you don't mix them.

### Query Engine Adaptation

The query engine currently operates on `RecordTree<SchRecord>`. The new `SchRecord` enum dispatches to v2 field structs. The query engine needs:

1. `SchRecord` to implement `SchPrimitive` (via delegation to inner type)
2. `get_property()` to return the same property names the selectors expect

The query parsing and matching logic doesn't change at all. Only the types at the boundary change.

### I/O

V2's `SchLibV2`, `SchDocV2`, `PcbLibV2`, `PcbDocV2` are promoted to `io::SchLib`, `io::SchDoc`, etc. They return records as v2 field structs (now the canonical types).

The I/O types build `RecordTree<SchRecord>` from the parsed records, restoring parent/child hierarchy. This is currently done in v1's `SchLib::open()` — the logic moves into the new `io::SchLib::open()`.

## How We Get There

This is not a phased migration. It's a replacement. The steps:

### Step 1: Scaffold the new structure

Create the `format/`, `sch/`, `pcb/` module structure. Move v2 code into `format/`. Create `SchRecord` enum and `SchPrimitive` trait implementations.

Build `io::SchLib::open()` that returns `RecordTree<SchRecord>` where `SchRecord` variants wrap v2 field structs.

**Test checkpoint:** V2 roundtrip tests pass using new module paths.

### Step 2: Adapt the query engine

Change `query/engine.rs` to work with the new `SchRecord` enum. The `SchPrimitive` trait provides `get_property()` which the selectors already use.

**Test checkpoint:** All 57+ query tests pass.

### Step 3: Adapt ops

Change `ops/` functions to accept the new I/O types. The field access patterns change (e.g., `record.designator` → `record.lib_reference` where names differ), but the logic is identical.

**Test checkpoint:** All ops tests pass.

### Step 4: Delete v1

Remove `records/`, `traits/`, `types/coord.rs`, `types/parameters.rs`, old `io/`, `api/`, and the entire `altium-format-derive` crate.

**Test checkpoint:** All tests pass. `cargo build --workspace` clean.

### Step 5: Add Document API (optional, future)

Build `Document` with `RawStore` + `ChangeSet` on top of the clean foundation. This is new functionality, not migration.

## Test Strategy

### Tests that survive unchanged (logic is independent of record types)
- query/ tests (57+) — after Step 2 adaptation, same assertions
- tree/ tests (8) — RecordTree is generic
- footprint/ tests (15) — PCB-only, PcbCoord unchanged
- v2 serializer tests (23) — these ARE the ground truth now

### Tests that get rewritten (same correctness property, different types)
- v1 record roundtrip tests → test v2 field struct roundtrip via SchSerializer
- v1 I/O tests → test new `io::SchLib::open()` with same assertions
- ops tests → same assertions, different input types

### Critical regression guards (NEVER lose these properties)
| Property | Current Test | New Test |
|----------|-------------|----------|
| Designator serializes as RECORD=34 | test_designator_roundtrip_record_id | Same, using v2 types |
| Pin PinConglomerate byte packing | pin_conglomerate_packing (v2) | Kept as-is |
| Parameter NotAllow* boolean inversion | parameter_inverted_booleans (v2) | Kept as-is |
| All pins survive component placement | test_placed_component_preserves_all_pins | Rewritten for new types |
| Implementation sub-records persist | test_implementation_child_records_persist_to_file | Rewritten for new types |
| CFB roundtrip preserves all records | v2_schlib_cfb_roundtrip | Kept as-is (module paths updated) |

### Tests that go away (tested redundant v1 format code)
- v1 FromParams/ToParams derive tests
- v1 FromBinary/ToBinary derive tests
- ParameterCollection parsing tests (replaced by AsciiSerializer tests)
- Coord (10K) unit tests (replaced by SchCoord/PcbCoord tests)

## What This Eliminates

- **altium-format-derive crate** — gone entirely
- **~3,000 LOC** of v1 record struct definitions (replaced by v2 field structs)
- **~1,500 LOC** of derive macro code
- **~800 LOC** of trait implementations (FromParams, ToParams, FromBinary, ToBinary)
- **Coordinate confusion** — one wrong type gone, two correct types remain
- **Two parallel I/O paths** — one path
- **Two sets of field names** — one set (v2's, from C# decompilation)
- **Bridging, adapters, migration phases** — none of this exists

## Open Questions

1. **Newtype vs type alias for SchPin etc.?** Type alias is zero-cost but can't add methods. Newtype adds `Deref` boilerplate but allows `impl SchPin { ... }`. Leaning toward newtype with `Deref<Target=PinData>`.

2. **Where do field structs live?** Currently `v2/fields/`. Options: (a) keep in `format/fields/` separate from export/import in `format/records/`, (b) merge into `format/records/` with their format functions. Leaning toward (a) — structs and serialization are separate concerns.

3. **RecordTree construction:** V1 builds the tree during I/O by tracking `owner_index`. V2's I/O returns flat lists. Tree construction code needs to move from v1's `io::SchLib` into the new `io::SchLib`. This is ~50 lines of straightforward index-based parent/child linking.

4. **ops/ coupling:** How many ops functions use v1 field names that differ from v2? Need to audit actual field access patterns. This determines how much ops code changes.
