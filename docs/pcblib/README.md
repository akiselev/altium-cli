# PcbLib Documentation

Reference documentation for implementing the PcbLib (PCB Footprint Library) parser in `crates/altium-format/`.

## Files

| File | Contents |
|------|----------|
| [cfb-structure.md](cfb-structure.md) | CFB (OLE Compound Binary) storage layout, stream inventory, and differences from PcbDoc |
| [fileheader.md](fileheader.md) | `/FileHeader` stream format: version identification and key token |
| [library-storage.md](library-storage.md) | `/Library/` storage: board defaults, layer stack, models, fonts, pad/via library |
| [footprint-data-stream.md](footprint-data-stream.md) | Per-footprint `Data` stream: pattern name block and packed binary primitives |
| [parameters-stream.md](parameters-stream.md) | Per-footprint `Parameters` stream: footprint metadata (pattern, height, description) |
| [sidecar-streams.md](sidecar-streams.md) | WideStrings, UniqueIDPrimitiveInformation, ExtendedPrimitiveInformation, PrimitiveGuids |
| [binary-primitives.md](binary-primitives.md) | PCB binary primitive record layouts for all object types found in PcbLib |
| [sectionkeys.md](sectionkeys.md) | `/SectionKeys` stream: mapping long footprint names to truncated CFB storage keys |
| [loading-pipeline.md](loading-pipeline.md) | Complete load pipeline in exact execution order |
| [shared-with-pcbdoc.md](shared-with-pcbdoc.md) | Overlap with PcbDoc format: what to share vs what differs |
| [coordinate-system.md](coordinate-system.md) | Internal units, coordinate encoding, colors |
| [enumerations.md](enumerations.md) | All enumerations used by PCB primitives (TObjectId, shapes, layers, etc.) |

## Quick orientation

A PcbLib file is a CFB (OLE Compound Binary / Structured Storage) container. The top-level
structure is:

- `/FileHeader` - library format identification string and key token
- `/SectionKeys` - optional mapping of long footprint names to truncated CFB storage keys
- `/FileVersionInfo/{Header,Data}` - file version history
- `/Library/` - library-wide data: board defaults, layer stack, 3D models, fonts, pad/via library
- `/<FootprintName>/` - one sub-storage per footprint containing binary primitives and metadata

Each footprint storage contains:
- `Data` - pattern name block followed by packed binary primitive records (`u8 type + u32 len + payload`)
- `Header` - record count (u32)
- `Parameters` - pipe-delimited footprint metadata (`|PATTERN=name|HEIGHT=value|...`)
- `WideStrings` - parameter-block format Unicode strings (**NOT** binary TLV like PcbDoc!)
- `PrimitiveGuids/{Header,Data}` - binary GUID table (24 bytes/entry)
- `UniqueIDPrimitiveInformation/{Header,Data}` - per-primitive unique IDs
- `ExtendedPrimitiveInformation/{Header,Data}` - per-primitive extended properties (rare)

The main parsing challenge compared to SchLib is that PcbLib uses **binary primitive records**
(not pipe-delimited text), so parsing requires knowing the exact byte layout of each object type.
The object types found in PcbLib footprints are: Arc(1), Pad(2), Via(3), Track(4), Text(5),
Fill(6), Region(11), and ComponentBody(12).

## Key differences from SchLib

| Aspect | SchLib | PcbLib |
|--------|--------|--------|
| Record format | Pipe-delimited text (`\|KEY=VALUE\|`) | Binary structs (little-endian packed) |
| Record dispatch | `RECORD=N` parameter key | `u8` object ID byte prefix |
| Record framing | 4-byte header: `flags(8b) \| size(24b)` | `u8 type + u32 length + payload` |
| Pin handling | Binary blocks (flags=0x01) mixed with text | All primitives are binary |
| Sidecar streams | 9 pin sidecar streams + Storage (images) | WideStrings + PrimitiveGuids + UniqueID + ExtendedPrimitiveInfo |
| Coordinate system | i16 DXP units (1 unit = 100,000 internal) | i32 internal units directly (10,000 = 1 mil) |
| Component nesting | OWNERINDEX tree | Flat list per footprint storage |
| Library-wide data | FileHeader (font table, component index) | Library/ storage (board defaults, models, layers) |
| Name mapping | SectionKeys (same concept) | SectionKeys (same concept) |

## Key differences from PcbDoc

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| Structure | Flat sections per primitive type | One storage per footprint, all types mixed |
| Sections | Arcs6/, Pads6/, Tracks6/, etc. | `<Footprint>/Data` (all types concatenated) |
| Pattern name | N/A (components reference footprints) | First block in Data stream |
| WideStrings | Binary TLV format (WideStrings6) | Parameter-block format (`\|ENCODEDTEXT0=...\|`) |
| Models | `/Models/` at root | `/Library/Models/` |
| Board data | `/Board6/Data` (full board settings) | `/Library/Data` (board defaults for library context) |
| Ownership | Component index in each primitive section | All primitives belong to the enclosing footprint |
