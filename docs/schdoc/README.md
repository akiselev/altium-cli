# SchDoc Documentation

Reference documentation for implementing the SchDoc (Schematic Document) parser in
`crates/altium-format/`.

## Files

| File | Contents |
|------|----------|
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
