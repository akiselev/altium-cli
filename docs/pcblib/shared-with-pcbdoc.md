# Shared with PcbDoc

PcbLib and PcbDoc share significant format infrastructure. This document identifies what
can be shared and what must differ between implementations.

## Fully shared (same code)

### Binary primitive record parsing

The binary layout of each primitive type (Arc, Pad, Via, Track, Text, Fill, Region,
ComponentBody) is **identical** between PcbDoc and PcbLib. The same `parse_arc()`,
`parse_pad()`, etc. functions handle both formats.

The only difference is the record framing context:
- PcbDoc: records appear in type-specific sections (Arcs6/Data has only type=1 records)
- PcbLib: records appear mixed in a single Data stream (any type can follow any other)

### TObjectId type dispatch

The `u8` type byte and the `TObjectId` enum are identical.

### Coordinate system

Same i32 internal units (10,000 = 1 mil). Same `Coord` type, same `CoordPoint` type.

### Common header

The 13-byte `PcbPrimitiveCommon` / `PcbCommonHeader` struct is identical.

### Enumerations

All PCB enumerations are shared: layer IDs, pad shapes, stack modes, flags, text
justification, region kinds, etc.

### SectionKeys

The SectionKeys stream format is identical to SchLib and PcbLib. Same parser handles both.

### Header/Data section pattern

The `u32 count Header + variable Data` pattern is identical. The same section-reading
infrastructure handles both PcbDoc sections and PcbLib library/footprint sub-storages.

### Parameter block parsing

The pipe-delimited `|KEY=VALUE|` parameter format and Windows-1252 encoding is identical.
Same parser for PcbDoc Board6/Components6/etc. and PcbLib Parameters/UniqueIDPrimitiveInformation.

### Length-prefixed string framing

The `u32 block_length + u8 string_length + string` framing used by PcbLib pattern name
blocks and Parameters stream is related to (but simpler than) the PcbDoc parameter block
framing which is `u32 block_length + NUL-terminated parameter string`.

### FileVersionInfo

Same format in both PcbDoc and PcbLib.

### PadViaLibrary

Same parameter format.

### LayerKindMapping

Same binary format.

### Textures

Same format.

### ModelsNoEmbed

Same format.

### 3D model storage

The Models/{Header,Data,0,1,...} structure is identical, though located at different
paths:
- PcbDoc: `/Models/`
- PcbLib: `/Library/Models/`

## Different between PcbDoc and PcbLib

### CFB structure

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| Top-level sections | One per primitive type (Arcs6/, Pads6/, ...) | One per footprint (<Name>/) |
| Board data | `/Board6/Data` | `/Library/Data` |
| Models location | `/Models/` | `/Library/Models/` |
| FileHeader format | `"PCB 5.0 Binary File"` + UTF-16LE | `"PCB 6.0 Binary Library File"` + ASCII |

### Record framing

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| Data stream | Single type per section | Mixed types, pattern name prefix |
| Type validation | Must match section (Arcs6 → type=1) | Any footprint-valid type allowed |
| Pattern name | N/A | First block in Data stream |

### WideStrings format

| Aspect | PcbDoc WideStrings6 | PcbLib WideStrings |
|--------|---------------------|---------------------|
| Encoding | Binary TLV (type tag + length + data) | Parameter blocks (`\|ENCODEDTEXT0=N,N,...\|`) |
| Types | 0x06 (ASCII/u8), 0x0C (ASCII/u32), 0x12 (UTF-16LE), 0x14 (UTF-8) | Comma-separated decimal bytes (UTF-8) |
| Scope | Board-level (all text primitives) | Per-footprint |

**These require completely separate parser implementations.**

### Sidecar stream scope

| Sidecar | PcbDoc | PcbLib |
|---------|--------|--------|
| UniqueIDPrimitiveInformation | Board-level (`/UniqueIDPrimitiveInformation/`) | Per-footprint (`/<Name>/UniqueIDPrimitiveInformation/`) |
| ExtendedPrimitiveInformation | Board-level | Per-footprint |
| PrimitiveGuids | Board-level | Per-footprint |

The data format within each sidecar stream is the same; only the CFB path differs.

### Ownership model

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| Component→Primitive | `component_index` in each primitive's common header | Implicit — all primitives in a footprint storage belong to that footprint |
| Net assignment | `net_index` in common header | Always 0/-1 (no nets in library context) |
| Polygon assignment | `polygon_index` in common header | Always 0 (no polygons in footprint context) |

### PcbDoc-only features

These exist only in PcbDoc, not PcbLib:
- `Board6/` section (full board settings, rules, nets)
- Net definitions (`Nets6/`)
- Component instances (`Components6/`)
- Polygon pours (`Polygons6/`)
- Design rules (`Rules6/`, `NewRules6/`)
- Class definitions (`Classes6/`)
- Connections/ratsnest (`Connections6/`, `FromTos6/`)
- Differential pairs (`DifferentialPairs6/`)
- Violations (`WaivedViolations/`)
- Embedded boards (`EmbeddedBoards6/`, `Embeddeds6/`)
- Constraint manager (`ConstraintManager/`)
- Various option sections (Placer, Router, DRC, Pin Swap)

### PcbLib-only features

These exist only in PcbLib, not PcbDoc:
- Per-footprint storages with mixed primitive types
- Pattern name block in Data stream
- Per-footprint Parameters stream
- `/Library/ComponentParamsTOC/`
- `/Library/EmbeddedFonts` (at library level; PcbDoc has `EmbeddedFonts6/`)

## Recommended shared code structure

```
altium-format/src/pcb/
  ├── primitives/          # Shared binary record parsers
  │   ├── arc.rs
  │   ├── pad.rs
  │   ├── via.rs
  │   ├── track.rs
  │   ├── text.rs
  │   ├── fill.rs
  │   ├── region.rs
  │   ├── component_body.rs
  │   └── common.rs        # PcbPrimitiveCommon, PcbFlags
  ├── models.rs            # 3D model parsing (shared)
  ├── sidecar.rs           # UniqueID, ExtendedPrimitiveInfo parsing (shared format)
  ├── parameters.rs        # Pipe-delimited parameter parsing (shared)
  ├── section_keys.rs      # SectionKeys parsing (shared with SchLib too)
  └── documents/
      ├── pcbdoc.rs        # PcbDoc-specific document structure
      ├── pcblib.rs        # PcbLib-specific document structure
      └── wide_strings.rs  # Both WideStrings formats (PcbDoc TLV + PcbLib param blocks)
```

The key design principle: **primitive parsing is shared, document structure is separate.**
