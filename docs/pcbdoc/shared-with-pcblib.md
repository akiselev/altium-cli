# Shared with PcbLib

PcbDoc and PcbLib share significant format infrastructure. This document identifies what
can be shared and what must differ between implementations.

## Fully shared (same code)

### Binary primitive record parsing

The binary layout of each primitive type (Arc, Pad, Via, Track, Text, Fill, Region,
ComponentBody) is **identical** between PcbDoc and PcbLib. The same parsing functions
handle both formats.

The only difference is the record framing context:
- PcbDoc: records appear in type-specific sections (Arcs6/Data has only type=1 records)
- PcbLib: records appear mixed in a single Data stream (any type can follow any other)

### TObjectId type dispatch

The `u8` type byte and the `TObjectId` enum are identical.

### Record framing

Both use `u8 object_id + u32 LE length + payload` framing. The high byte of the length
field may contain flags in both formats; mask with `BLOCK_SIZE_MASK`.

### Common header

The 13-byte common header at the start of every primitive is identical:
- `u8 layer` (offset 0)
- `u16 flags` / `PcbFlags` (offset 1-2)
- `u16 net_index` (offset 3-4, 0xFFFF = none)
- `u16 polygon_index` (offset 5-6, 0xFFFF = none)
- `u16 component_index` (offset 7-8, 0xFFFF = none)
- `u16 coordinate_index` (offset 9-10, 0xFFFF = none)
- `u16 dimension_index` (offset 11-12, 0xFFFF = none)

In PcbLib context, `net_index` is always 0xFFFF and `component_index` is implicit (all
primitives belong to the enclosing footprint storage). Both `coordinate_index` and
`dimension_index` are always 0xFFFF in observed data.

### Coordinate system

Same i32 internal units (10,000 = 1 mil). Same `Coord` type, same `CoordPoint` type.

### Enumerations

All PCB enumerations are shared: layer IDs, pad shapes, stack modes, flags, text kinds,
region kinds, dimension kinds, etc.

### Header/Data section pattern

The `u32 count Header + variable Data` pattern is identical. The same section-reading
infrastructure handles both PcbDoc sections and PcbLib library/footprint sub-storages.

### Parameter block parsing

The pipe-delimited `|KEY=VALUE|` parameter format and Windows-1252 encoding is identical.
Same parser for PcbDoc Board6/Components6/Nets6/etc. and PcbLib Parameters/UniqueID streams.

### 3D model storage

The `Models/{Header,Data,0,1,...}` structure is identical, though located at different paths:
- PcbDoc: `/Models/`
- PcbLib: `/Library/Models/`

### Sidecar stream formats

UniqueIDPrimitiveInformation, ExtendedPrimitiveInformation, and PrimitiveGuids use the
**same data format** in both PcbDoc and PcbLib. Only the CFB path differs:
- PcbDoc: board-level (`/UniqueIDPrimitiveInformation/`)
- PcbLib: per-footprint (`/<FootprintName>/UniqueIDPrimitiveInformation/`)

### FileVersionInfo

Same format in both PcbDoc and PcbLib.

### PadViaLibrary, LayerKindMapping, Textures, ModelsNoEmbed

Same parameter/binary formats in both.

### SectionKeys

Same format as PcbLib (and SchLib). Same parser handles all three.

## Different between PcbDoc and PcbLib

### CFB structure

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| Top-level sections | One per primitive type (Arcs6/, Pads6/, ...) | One per footprint (<Name>/) |
| Board data | `/Board6/Data` (~100KB, ~2700 keys) | `/Library/Data` (library defaults) |
| Models location | `/Models/` at root | `/Library/Models/` |
| EmbeddedFonts | `EmbeddedFonts6/` at root | `/Library/EmbeddedFonts` |

### FileHeader

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| Legacy stream | `/FileHeader` (UTF-16LE, 24 bytes) | None |
| V6 stream | `/FileHeaderSix` (pascal-block, 75 bytes) | `/FileHeader` (pascal-block, 53 bytes) |
| Version string | `"PCB 6.0 Binary File"` | `"PCB 6.0 Binary Library File"` |
| UniqueID | GUID in braces (38 chars) | 8-char uppercase alpha token |

### WideStrings format (CRITICAL)

PcbDoc and PcbLib use **completely different** WideStrings formats. These require separate
parser implementations.

| Aspect | PcbDoc WideStrings6 | PcbLib WideStrings |
|--------|---------------------|---------------------|
| Format | Binary: `[u32 index][u32 byte_length][UTF-16LE data]` | Parameter blocks: `\|ENCODEDTEXT0=N,N,...\|` |
| Encoding | UTF-16LE with NUL terminator | Comma-separated decimal bytes (UTF-8) |
| Scope | Board-level (all text primitives in one table) | Per-footprint |
| Stream path | `/WideStrings6/{Header,Data}` | `/<FootprintName>/WideStrings` |

**Note:** The existing `docs/dxp/pcb-files.md` describes a binary TLV format with type tags
(0x06, 0x0C, 0x12, 0x14) for PcbDoc WideStrings6. However, actual AD26 files use the simpler
`index + byte_length + UTF-16LE` format described above. The TLV format may apply to older
format versions or specific file variants.

### Record framing context

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| Data stream | Single type per section | Mixed types, pattern name prefix |
| Type validation | Must match section (Arcs6 → type=1) | Any footprint-valid type allowed |
| Pattern name | N/A | First block in Data stream |

### Ownership model

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| Component→Primitive | `component_index` in common header | Implicit (all primitives belong to enclosing footprint) |
| Net assignment | `net_index` in common header | Always 0/-1 (no nets in library context) |
| Polygon assignment | `polygon_index` in common header | Always 0 (no polygons in footprint context) |
| Ownership graph | Built post-load via SetIndexes (6 cross-reference indices) | Implicit from CFB hierarchy |

### PcbDoc-only sections

These exist only in PcbDoc, not PcbLib:
- `Board6/` -- full board settings, layer stack, grids, outline
- `Nets6/` -- net definitions
- `Components6/` -- component instances (designator, pattern, placement)
- `Polygons6/` -- polygon pour definitions
- `Rules6/`, `NewRules6/` -- design rules (prefixed parameter format)
- `Classes6/` -- object class definitions
- `Connections6/`, `FromTos6/` -- connections and ratsnest
- `DifferentialPairs6/` -- differential pair definitions
- `Dimensions6/`, `Coordinates6/` -- annotations (prefixed parameter format)
- `EmbeddedBoards6/`, `Embeddeds6/` -- embedded boards
- `WaivedViolations/` -- waived DRC violations
- `ConstraintManager/` -- constraint manager data
- `SmartUnions/`, `UnionRelations/`, `UnionNames/` -- union management
- `SignalClasses/` -- signal class definitions
- `PinPairsSection/` -- pin pair definitions
- `PrimitiveParameters/` -- component-level imported parameters
- `SplitPlaneRegions6/` -- split plane region primitives
- `ShapeBasedRegions6/`, `ShapeBasedComponentBodies6/` -- shape-based variants
- `BoardRegions/`, `Texts/` -- legacy sections
- Options sections: Advanced Placer/Router, DRC, Pin Swap

### PcbLib-only features

These exist only in PcbLib, not PcbDoc:
- Per-footprint storages with mixed primitive types
- Pattern name block as first block in Data stream
- Per-footprint `Parameters` stream (footprint metadata)
- `/Library/ComponentParamsTOC/` (component parameter table of contents)
- `/Library/EmbeddedFonts` (at library level)

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
  │   └── common.rs        # Common header, PcbFlags
  ├── models.rs            # 3D model parsing (shared)
  ├── sidecar.rs           # UniqueID, ExtendedPrimitiveInfo, PrimitiveGuids (shared format)
  ├── parameters.rs        # Pipe-delimited parameter parsing (shared)
  ├── section_keys.rs      # SectionKeys parsing (shared with SchLib)
  └── documents/
      ├── pcbdoc.rs        # PcbDoc-specific: section registry, Board6, ownership graph
      ├── pcblib.rs        # PcbLib-specific: footprint storages, pattern names
      └── wide_strings.rs  # Both WideStrings formats (PcbDoc binary + PcbLib param blocks)
```

The key design principle: **primitive parsing is shared, document structure is separate.**
