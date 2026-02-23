# Shared with SchLib

Analysis of overlap between SchDoc and SchLib implementations, with recommendations for
abstraction and code sharing.

## Fully shared infrastructure (implement once)

These components are byte-for-byte identical between SchDoc and SchLib. They should be
implemented as shared modules in `altium-format`.

### Block framing

The 4-byte block header format is universal:
- Lower 24 bits = payload size
- Upper 8 bits = flags (0x00 = parameter text, 0x01 = binary)

One shared `read_block()` / `write_block()` implementation.

### Parameter text parsing

Pipe-delimited `|key=value|` format, Windows-1252 encoding, NUL-terminated. The
`parse_param_records()` function is identical.

One shared `ParameterCollection` / `FromParams` / `ToParams` trait system.

### DXP fractional coordinate encoding

`raw_value = integer_part * 100,000 + fractional_part`. The `cBaseUnit = 100000` constant.

One shared `Coord` type with `from_dxp_frac(integer, frac)` constructor.

### Color encoding

Win32 COLORREF `0x00BBGGRR` as i32.

One shared `Color` type.

### Font table

Identical format: `FontIdCount`, `Size{N}`, `FontName{N}`, `Bold{N}`, `Italic{N}`, etc.

One shared `FontTable` struct with `from_params()` / `to_params()`.

### Storage stream (embedded images)

Identical format: `Icon storage` header + 0xD0-tagged compressed entries.

One shared `EmbeddedStorage` reader/writer.

### Base types

`SchPrimitiveBase` and `SchGraphicalBase` have the same fields. Note: SchDoc records
commonly have `INDEXINSHEET` which is typically absent in SchLib, but the field can
exist in the base type with a default of -1.

### Derive macros

All `#[altium(param = "KEY")]`, `#[altium(frac = "KEY_FRAC")]`,
`#[altium(indexed_coords)]`, `#[altium(flatten)]` macros work identically.

## Shared record types (same fields, same serialization)

These record types appear in both SchDoc and SchLib with identical parameter text format.
They should share a single Rust struct definition.

| RECORD | Type | Notes |
|--------|------|-------|
| 4 | SchLabel | Identical |
| 5 | SchBezier | Identical |
| 6 | SchPolyline | Identical |
| 7 | SchPolygon | Identical |
| 8 | SchEllipse | Identical |
| 9 | SchPie | Identical |
| 11 | SchEllipticalArc | Identical |
| 12 | SchArc | Identical |
| 13 | SchLine | Identical |
| 14 | SchRectangle | Identical |
| 28 | SchTextFrame | Identical |
| 30 | SchImage | Identical |
| 34 | SchDesignator | Identical |
| 41 | SchParameter | Identical |
| 44 | SchImplementationList | Identical |
| 45 | SchImplementation | Similar (SchDoc may omit database link fields; use optional/default) |
| 46 | SchImplementationMap | Identical |
| 47 | SchMapDefiner | Identical |
| 48 | SchImplementationParameters | Identical |

## Records with format differences

### RECORD=1: SchComponent

The SchComponent record has additional fields in SchDoc vs SchLib:

| Field | SchDoc | SchLib |
|-------|--------|--------|
| `DESIGNITEMID` | Present | Absent |
| `ALLPINCOUNT` | Present | Absent |
| `NOTUSEDBTABLENAME` | Present | Absent |
| `ORIENTATION` | Integer 0-3 (RotationBy90) | Integer 0-3 (RotationBy90) — same encoding |
| `COMPONENTDESCRIPTION` | Optional | Present |
| `LIBREFERENCE` | Present | Present (identical) |

Recommendation: Use a single `SchComponent` struct with optional fields for the
SchDoc-specific additions. Orientation uses the same RotationBy90 encoding in both formats
(confirmed: both FileFormatV4 and FileFormatV5 call `Export_RotationBy90`/`Import_RotationBy90`).

### RECORD=2: SchPin

Pins have fundamentally different serialization between SchDoc and SchLib:

| Aspect | SchDoc | SchLib |
|--------|--------|--------|
| Block flags | 0x00 (parameter text) | 0x01 (binary) |
| Coordinate encoding | DXP fractional (full precision) | i16 truncated + PinFrac sidecar |
| Text encoding | Parameter text keys | Binary length-prefixed strings |
| Sidecar streams | None | 9 pin sidecar streams |
| Additional fields | None beyond params | PinWideText overrides, PinMiscData, etc. |

Recommendation: The `SchPin` domain struct should be the same, but deserialization needs
two code paths:
1. `SchPin::from_params()` for SchDoc
2. `SchPin::from_binary()` + sidecar merging for SchLib

The serialized field set is identical -- same pin properties either way:
`Name`, `Designator`, `Electrical`, `PinConglomerate`, `PinLength`, `Location`,
`FormalType`, `SwapIdPart`, `Color`, etc.

## SchDoc-only record types

These exist only in SchDoc and need new struct definitions:

| RECORD | Type | Complexity |
|--------|------|-----------|
| 31 | SchSheet | High (font table, many settings) |
| 39 | SchTemplate | Low (just FileName) |
| 27 | SchWire | Medium (indexed coords, like Polyline) |
| 26 | SchBus | Medium (same as Wire) |
| 25 | SchNetLabel | Low (Text + Location + FontID) |
| 17 | SchPowerObject | Low (Text + Style + Orientation) |
| 18 | SchPort | Medium (Name, IOType, Style, dimensions) |
| 22 | SchNoConnect | Low (Symbol, SuppressAll, IsActive) |
| 29 | SchJunction | Minimal (just Location + Color) |
| 15 | SchSheetSymbol | Medium (Location+Corner, IsSolid, SheetName) |
| 16 | SchSheetEntry | Low (Name, IOType, Side, Style) |
| 43 | SchCompileMask | Low (Name, Orientation) |
| 209 | SchHyperlink | Medium (Text, Author, Corner, formatting) |
| 225 | SchDashedRectangle | Medium (indexed coords, LineStyle) |

## SchLib-only features (not needed for SchDoc)

These SchLib features have no SchDoc equivalent:

- Binary pin format (flags=0x01 blocks)
- 9 pin sidecar streams (PinFrac, PinDesc, PinMiscData, etc.)
- Per-component CFB sub-storages
- SectionKeys mapping
- Alias/Redirection system
- Library header (HEADER, Weight, CompCount, LibRef{N}, etc.)
- Component index in FileHeader

## Recommended module structure

```
crates/altium-format/src/sch/
    mod.rs              // Re-exports
    types.rs            // Shared types: SchPrimitiveBase, SchGraphicalBase
    enums.rs            // Shared enums: PinElectricalType, LineWidth, etc.
    font.rs             // FontTable
    coord.rs            // DXP fractional coordinate helpers
    color.rs            // Color type
    block.rs            // Block framing (read/write)
    params.rs           // Parameter text parsing
    storage.rs          // Embedded object storage (Storage stream)
    records/
        mod.rs          // SchRecord enum dispatch
        component.rs    // RECORD=1 (SchDoc + SchLib)
        pin.rs          // RECORD=2 (text + binary deserialization)
        label.rs        // RECORD=4
        polyline.rs     // RECORD=6, also used by Wire (27), Bus (26)
        polygon.rs      // RECORD=7
        arc.rs          // RECORD=12
        rectangle.rs    // RECORD=14
        line.rs         // RECORD=13
        // ... other shared records
        wire.rs         // RECORD=27 (SchDoc-only)
        netlabel.rs     // RECORD=25 (SchDoc-only)
        power.rs        // RECORD=17 (SchDoc-only)
        junction.rs     // RECORD=29 (SchDoc-only)
        noconnect.rs    // RECORD=22 (SchDoc-only)
        sheet.rs        // RECORD=31 (SchDoc-only)
        // ... other SchDoc-only records
    documents/
        schdoc.rs       // SchDoc container (loading/saving pipeline)
        schlib.rs       // SchLib container (loading/saving pipeline)
```

The `records/` module contains all record type structs shared between both document types.
The `documents/` module contains the document-specific loading/saving pipelines.
