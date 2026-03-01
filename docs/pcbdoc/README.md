# PcbDoc Documentation

Reference documentation for implementing the PcbDoc (PCB Board Document) parser in
`crates/altium-format/`.

## Files

| File | Contents |
|------|----------|
| [cfb-structure.md](cfb-structure.md) | CFB (OLE Compound Binary) storage layout: all 42+ section storages and root streams |
| [fileheader.md](fileheader.md) | `/FileHeader` (legacy UTF-16LE) and `/FileHeaderSix` (pascal-block) streams |
| [board-section.md](board-section.md) | `Board6` section: board settings, layer stack (4 generations), outline vertices, grids |
| [binary-primitives.md](binary-primitives.md) | Binary record layouts for all primitive types (Arc, Pad, Via, Track, Text, Fill, Region, ComponentBody) |
| [parameter-sections.md](parameter-sections.md) | Parameter-format sections: Nets6, Components6, Polygons6, Rules6, Dimensions6, and 20+ others |
| [sidecar-streams.md](sidecar-streams.md) | WideStrings6 (binary index+UTF-16LE), UniqueID, ExtendedPrimitiveInfo, PrimitiveGuids, PrimitiveParameters |
| [loading-pipeline.md](loading-pipeline.md) | Complete load (7 phases) and save (5 phases) pipelines in exact execution order |
| [enumerations.md](enumerations.md) | All enumerations: TObjectId, layers, pad shapes, TRuleKind (70 values), TStorageFeature, and more |
| [shared-with-pcblib.md](shared-with-pcblib.md) | Overlap analysis with PcbLib: shared primitives, different document structure |
| [serialization.md](serialization.md) | Save pipeline (5 phases), section export, sidecar build, implementation checklist |
| [stream_table.md](stream_table.md) | Complete section name table: all ~166 CFB storages, Delphi addresses, DRC violations, TObjectId, TStorageFeature |

## Quick orientation

A PcbDoc file is a CFB (OLE Compound Binary / Structured Storage) container. Unlike PcbLib
(which has per-footprint sub-storages), a PcbDoc is a **flat collection of section storages**
at the root level -- one per primitive type, plus board settings, nets, components, rules,
and sidecar streams:

```
Root Storage
 +-- FileHeader              (legacy V5 UTF-16LE identification)
 +-- FileHeaderSix           (V6 pascal-block: version string + f64 + GUID)
 +-- Board6/                 (board-level settings: ~2700 parameter keys)
 +-- Arcs6/                  (binary arc records)
 +-- Pads6/                  (binary pad records)
 +-- Vias6/                  (binary via records)
 +-- Tracks6/                (binary track records)
 +-- Texts6/                 (binary text records)
 +-- Fills6/                 (binary fill records)
 +-- Regions6/               (binary region records)
 +-- ComponentBodies6/       (binary component body records)
 +-- Nets6/                  (parameter blocks: net definitions)
 +-- Components6/            (parameter blocks: component instances)
 +-- Polygons6/              (parameter blocks: polygon pour definitions)
 +-- Rules6/                 (prefixed parameter blocks: design rules)
 +-- Dimensions6/            (prefixed parameter blocks: dimension annotations)
 +-- Models/                 (3D model metadata + embedded STEP blobs)
 +-- WideStrings6/           (binary sidecar: Unicode text for primitives)
 +-- UniqueIDPrimitiveInformation/  (parameter sidecar: per-primitive IDs)
 +-- ... (42+ total sections)
```

Each section storage contains a `Header` stream (u32 record count) and a `Data` stream
(records in the format appropriate for that section type).

The ownership model uses **cross-reference indices** stored in each primitive's common
header: `net_index`, `polygon_index`, `component_index`, `coordinate_index`,
`dimension_index` (all u16, 0xFFFF = none) pointing into the corresponding parameter
sections. This is fundamentally different from SchDoc's OWNERINDEX tree model.

## Key differences from PcbLib

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| CFB layout | Flat sections at root (one per type) | Hierarchical (per-footprint storages) |
| Primitive grouping | Type-specific sections (Arcs6/ has only arcs) | Mixed in single Data stream per footprint |
| FileHeader | Legacy UTF-16LE (`/FileHeader`) + V6 (`/FileHeaderSix`) | Single V6 pascal-block (`/FileHeader`) |
| UniqueID format | GUID in braces (38 chars) | 8-char alpha token |
| WideStrings format | Binary `[u32 index][u32 len][UTF-16LE]` | Parameter blocks (`\|ENCODEDTEXT0=...\|`) |
| Board/Library data | `/Board6/Data` (~100KB, ~2700 keys) | `/Library/Data` (smaller, library defaults) |
| Models location | `/Models/` at root | `/Library/Models/` |
| Ownership | Explicit indices (net, component, polygon) in common header | Implicit (all primitives belong to enclosing footprint) |
| Nets, rules, classes | Full sections (Nets6, Rules6, Classes6, etc.) | Not present |
| Pattern name block | N/A | First block in each footprint Data stream |

## Key differences from SchDoc

| Aspect | PcbDoc | SchDoc |
|--------|--------|--------|
| Record format | Binary structs (little-endian packed) | Parameter text (`\|KEY=VALUE\|`) |
| Record dispatch | `u8` object ID byte prefix | `RECORD=N` parameter key |
| Coordinate system | i32 internal units (10,000 = 1 mil) | i16 DXP units (1 unit = 100,000 internal) |
| Ownership | Cross-reference indices (net, component, polygon) | OWNERINDEX flat tree |
| Sidecar streams | WideStrings6, UniqueID, ExtendedPrimitiveInfo, PrimitiveGuids | None (all data in FileHeader stream) |
| Section structure | 42+ type-specific sections | 3 flat streams (FileHeader, Additional, Storage) |

## Shared infrastructure with PcbLib

The following components should be shared between PcbDoc and PcbLib implementations:

1. **Binary primitive parsing** -- identical record layouts for all 8+ primitive types
2. **TObjectId dispatch** -- same `u8` type byte enum
3. **Common header** -- same 13-byte header (layer, flags, indices)
4. **Coordinate system** -- same i32 internal units (10,000 = 1 mil)
5. **Parameter block parsing** -- same pipe-delimited `|KEY=VALUE|` format
6. **Section Header/Data pattern** -- same u32 count + data stream structure
7. **SectionKeys** -- same format as PcbLib and SchLib
8. **3D model storage** -- same Models/{Header,Data,0,1,...} structure
9. **Sidecar formats** -- UniqueID, ExtendedPrimitiveInfo, PrimitiveGuids use same formats
10. **Enumerations** -- all PCB enums shared (layers, shapes, modes, flags)

**NOT shared**: WideStrings format (binary in PcbDoc vs parameter blocks in PcbLib),
document structure, and board-level sections.

See [shared-with-pcblib.md](shared-with-pcblib.md) for the full overlap analysis.
