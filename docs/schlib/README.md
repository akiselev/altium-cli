# SchLib Documentation

Reference documentation for implementing the SchLib (Schematic Library) parser in `crates/altium-format/`.

## Files

| File | Contents |
|------|----------|
| [cfb-structure.md](cfb-structure.md) | CFB (OLE Compound Binary) storage layout and block encoding |
| [fileheader.md](fileheader.md) | `/FileHeader` stream format: library header, font table, component index |
| [component-data-stream.md](component-data-stream.md) | Per-component `Data` stream: block sequence, record types, OwnerIndex |
| [binary-pin-format.md](binary-pin-format.md) | Binary pin record layout (flags=0x01 blocks) |
| [pin-sidecar-streams.md](pin-sidecar-streams.md) | All 9 pin sidecar streams: formats, import order, presence patterns |
| [record-types.md](record-types.md) | Parameter text record field definitions for all record types |
| [aliases-and-sectionkeys.md](aliases-and-sectionkeys.md) | SectionKeys stream, alias system, and redirection streams |
| [loading-pipeline.md](loading-pipeline.md) | Complete load and save pipeline in exact execution order |
| [enumerations.md](enumerations.md) | All enumerations used by SchLib record types |
| [coordinate-system.md](coordinate-system.md) | Internal units, DXP fractional encoding, binary pin coordinates, colors |

## Quick orientation

A SchLib file is a CFB (OLE Compound Binary / Structured Storage) container. The top-level
structure is:

- `/FileHeader` - library-wide header, font table, and component index
- `/Storage` - global embedded binary objects (images)
- `/SectionKeys` - maps long component names to short CFB storage keys
- `/<ComponentKey>/Data` - one sub-storage per component containing all record blocks
- `/<ComponentKey>/PinFrac`, `PinDesc`, etc. - optional sidecar streams for pin data

The main parsing challenge is the two-tier block format inside each `Data` stream:
parameter text blocks (flags=0x00, pipe-delimited key=value) and binary blocks
(flags=0x01, raw binary, always pins). See [component-data-stream.md](component-data-stream.md)
and [binary-pin-format.md](binary-pin-format.md).

The loading pipeline runs in three phases: base warehouse (structural records), extended
warehouse (embedded images and pin sidecar data), and additional warehouse (extended
per-component records). See [loading-pipeline.md](loading-pipeline.md) for the full order.
