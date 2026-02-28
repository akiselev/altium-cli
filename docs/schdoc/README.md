# SchDoc Documentation

Reference documentation for the SchDoc (Schematic Document) format and implementation in
`crates/altium-format/`.

## Implementation Status

| Layer | Status |
|-------|--------|
| File parsing (open/save/roundtrip/validate/render) | **Done** |
| High-level API (types, read path, write path, CRUD) | **Not started** — see [plan.md](plan.md) |
| Spec language integration | **Not started** — deferred |
| Dump command | **Not started** — depends on API |

## Files

| File | Contents |
|------|----------|
| [plan.md](plan.md) | **Implementation plan** for high-level API, dump command, spec integration, UniqueId identity architecture (§10) |
| [cfb-structure.md](cfb-structure.md) | CFB (OLE Compound Binary) storage layout for SchDoc files |
| [fileheader-stream.md](fileheader-stream.md) | `FileHeader` stream: document header, sheet record, font table, all content |
| [additional-stream.md](additional-stream.md) | `Additional` stream: supplementary records (RECORD=225 dashed rectangles) |
| [storage-stream.md](storage-stream.md) | `Storage` stream: embedded binary objects (images) |
| [record-types.md](record-types.md) | Parameter text record field definitions for all record types |
| [loading-pipeline.md](loading-pipeline.md) | Complete load and save pipeline in exact execution order |
| [enumerations.md](enumerations.md) | All enumerations used by SchDoc record types |
| [coordinate-system.md](coordinate-system.md) | Internal units, DXP fractional encoding, colors |
| [shared-with-schlib.md](shared-with-schlib.md) | Overlap analysis with SchLib: shared types, differences, abstraction strategy |

## Quick orientation

A SchDoc file is a CFB (OLE Compound Binary / Structured Storage) container. The structure
is **much simpler** than SchLib -- a flat list of records rather than per-component
storages:

```
Root Storage
 +-- FileHeader     (document header + ALL schematic records in a single stream)
 +-- Additional     (supplementary records: RECORD=225 dashed rectangles)
 +-- Storage        (embedded binary objects: images)
```

Optional streams (may or may not be present):
```
 +-- ObjectDefinitions
 +-- ReuseBlockInfos
 +-- ReuseBlocks / ReuseBlocksV2
 +-- HarnessConnectionPointConnector
 +-- Files
```

All records use **parameter text format** (pipe-delimited `|key=value|` pairs). Unlike
SchLib, there are **no binary pin blocks** -- pins use the same parameter text format as
all other records (RECORD=2 with flags=0x00).

The ownership model uses OWNERINDEX as a 0-based index into the flat global record list.
The first content record (index 0 after the header) is always the sheet (RECORD=31). All
component children have OWNERINDEX pointing to their parent component's absolute index.

## Key differences from SchLib

| Aspect | SchDoc | SchLib |
|--------|--------|--------|
| CFB layout | Flat (3 streams) | Hierarchical (per-component storages) |
| Pin format | Parameter text (RECORD=2, flags=0x00) | Binary (flags=0x01, first byte 0x02) |
| Pin sidecar streams | None | 9 streams (PinFrac, PinDesc, etc.) |
| OwnerIndex scope | Global (absolute index in flat list) | Relative (per-component section) |
| Sheet record | RECORD=31 (always first content) | Not present (library header instead) |
| SectionKeys/Aliases | Not applicable | Required for long component names |
| SchDoc-only records | Wire, Bus, NetLabel, Port, PowerObject, Junction, NoConnect, SheetSymbol, SheetEntry, Template, CompileMask | Not present |
| Additional stream | RECORD=225 dashed rectangles | Not present |
| Component RECORD=1 | Has DesignItemId, AllPinCount, Orientation | Has LIBREFERENCE, COMPONENTDESCRIPTION |

## Shared infrastructure with SchLib

The following components should be shared between SchDoc and SchLib implementations:

1. **Block framing** -- identical 4-byte header (24-bit size + 8-bit flags)
2. **Parameter text parsing** -- pipe-delimited key=value, Windows-1252
3. **Coordinate system** -- DXP fractional encoding (integer * 100,000 + frac)
4. **Color encoding** -- Win32 COLORREF 0x00BBGGRR
5. **Font table** -- identical format (FontIdCount, SizeN, FontNameN, etc.)
6. **Base types** -- SchPrimitiveBase, SchGraphicalBase
7. **Shared record types** -- Label, Polyline, Polygon, Arc, Rectangle, Line, Ellipse,
   Bezier, Pie, EllipticalArc, TextFrame, Image, Designator, Parameter, Implementation*
8. **Storage stream** -- identical embedded object format
9. **Enumerations** -- PinElectricalType, PinSymbol, LineWidth, LineStyle, TextJustification, etc.

See [shared-with-schlib.md](shared-with-schlib.md) for the full overlap analysis.

## Record ownership hierarchy

Records form a tree via OWNERINDEX (0-based absolute index into the flat record list).
Records are written in **depth-first order** during save — parent before children.

### Complete ownership tree

```
Sheet (RECORD=31) [always index 0]
├── Template (RECORD=39) [always index 1]
│   └── Graphics: Image(30), Label(4), ...
│
├── Component (RECORD=1) [CONTAINER]
│   ├── Designator (34) [field object, always 1]
│   ├── Parameter "Comment" (41) [field object, always 1]
│   ├── Pin (2) [0..N]
│   ├── Parameter (41) [0..N]
│   ├── ImplementationList (44) [0..1, CONTAINER]
│   │   └── Implementation (45) [1..N, CONTAINER]
│   │       ├── ImplementationMap (46) [field object, CONTAINER]
│   │       │   └── MapDefiner (47) [0..N]
│   │       └── ParameterList (48) [0..1]
│   └── Graphics: Line(13), Rectangle(14), Polyline(6), Polygon(7),
│       Ellipse(8), Arc(12), EllipticalArc(11), Pie(9), Bezier(5),
│       RoundRectangle(10), Image(30), Label(4), Symbol(3), TextFrame(28)
│
├── SheetSymbol (RECORD=15) [CONTAINER]
│   ├── SheetName (32) [field object, always 1]
│   ├── SheetFileName (33) [field object, always 1]
│   ├── SheetEntry (16) [0..N]
│   └── Parameter (41) [0..N]
│
├── ParameterSet (RECORD=43) [CONTAINER]
│   └── Parameter (41) [0..N]
│
├── HarnessConnector (RECORD=215) [CONTAINER]
│   ├── HarnessEntry (216) [0..N]
│   ├── HarnessConnectorType (217) [0..1]
│   └── Parameter (41) [0..N]
│
├── Top-level leaf records (OWNERINDEX=0):
│   Wire(27), Bus(26), NetLabel(25), PowerObject(17), Port(18),
│   NoConnect(22), Junction(29), BusEntry(37), Note(209), Probe(210),
│   CompileMask(211), SignalHarness(218), Label(4), Line(13),
│   Rectangle(14), Polyline(6), ..., Parameter(41)
│
└── [Additional stream]: Blanket (RECORD=225)
```

### Key observations

- **Parameter (RECORD=41) is polymorphic**: appears as child of Sheet, Component,
  SheetSymbol, ParameterSet, or HarnessConnector. Its role depends on its parent.
- **Field objects** (Designator, Comment, SheetName, SheetFileName, ImplementationMap)
  are intrinsic to their parent — always exactly one.
- **Ordering within each container** has semantic meaning: RECORD ≤ 225 preserves
  insertion order; RECORD > 225 sorts by type ascending.
- **Graphical primitives** can appear at two levels: directly on Sheet (annotations)
  or as children of Component (symbol graphics).

### Example flat record list

```
Record 0: Sheet (RECORD=31)                    <- root
Record 1: Template (RECORD=39)                 <- OWNERINDEX=0
Record 2:   Image (RECORD=30)                  <- OWNERINDEX=1 (template-owned)
Record 3: Component "U1" (RECORD=1)            <- OWNERINDEX=0
Record 4:   Pin (RECORD=2)                     <- OWNERINDEX=3
Record 5:   Pin (RECORD=2)                     <- OWNERINDEX=3
Record 6:   Designator "U1" (RECORD=34)        <- OWNERINDEX=3
Record 7:   Parameter "Value" (RECORD=41)      <- OWNERINDEX=3
Record 8:   ImplementationList (RECORD=44)     <- OWNERINDEX=3
Record 9:     Implementation (RECORD=45)       <- OWNERINDEX=8
Record 10:      ImplementationMap (RECORD=46)  <- OWNERINDEX=9
Record 11:        MapDefiner (RECORD=47)       <- OWNERINDEX=9
Record 12:      ParameterList (RECORD=48)      <- OWNERINDEX=9
Record 13: Wire (RECORD=27)                    <- OWNERINDEX=0
Record 14: NetLabel "VCC" (RECORD=25)          <- OWNERINDEX=0
Record 15: Junction (RECORD=29)                <- OWNERINDEX=0
```

## High-level API design

The API reflects the tree, not the flat serialization. A single ordered
`Vec<SheetObject>` enum preserves document ordering while grouping children
inside their parents.

```
SchDocSheet
├── fonts, grid settings, display settings, ...
├── template: Template { file_name, children: Vec<Graphic> }
└── objects: Vec<SheetObject>
    ├── SheetObject::Component(SchDocComponent)
    │   ├── designator, lib_reference, location, orientation, ...
    │   └── children: Vec<ComponentChild>
    │       ├── ComponentChild::Pin(Pin)           ← reused from SchLib API
    │       ├── ComponentChild::Parameter(Parameter) ← reused
    │       ├── ComponentChild::Graphic(Graphic)     ← reused
    │       └── ComponentChild::FootprintMap(FootprintMap) ← reused
    ├── SheetObject::Wire(Wire)
    ├── SheetObject::NetLabel(NetLabel)
    ├── SheetObject::PowerObject(PowerObject)
    ├── SheetObject::SheetSymbol(SheetSymbol)
    │   └── children: Vec<SheetSymbolChild>
    │       ├── SheetSymbolChild::Entry(SheetEntry)
    │       └── SheetSymbolChild::Parameter(Parameter)
    ├── SheetObject::Graphic(Graphic)          ← sheet-level annotation
    ├── SheetObject::Parameter(Parameter)      ← sheet-level parameter
    └── ... (Junction, NoConnect, Port, Bus, BusEntry, Note, etc.)
```

See [plan.md](plan.md) for the full type definitions and implementation strategy.
