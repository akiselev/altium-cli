> **Authoritative reference**: See [../../dxp/sch-files.md](../../dxp/sch-files.md)
> for the canonical format specification. This document covers SchDoc-specific details.

# CFB Structure

SchDoc files use the OLE Compound Binary (CFB / Structured Storage) format. The structure
is flat compared to SchLib -- there are no per-component sub-storages.

```
Root Storage
 |
 +-- FileHeader                  (document header + ALL schematic content records)
 +-- Additional                  (supplementary records: RECORD=225 dashed rectangles)
 +-- Storage                     (embedded binary objects: images)
 |
 +-- ObjectDefinitions           (optional - object definitions)
 +-- ReuseBlockInfos             (optional - reuse block metadata)
 +-- ReuseBlocks                 (optional - reuse block data)
 +-- ReuseBlocksV2               (optional - reuse block data v2)
 +-- HarnessConnectionPointConnector  (optional - harness connector data)
 +-- Files                       (optional - embedded file data)
```

## Always-present streams

Every SchDoc file has exactly **3 streams** and **0 storages**:

| Stream | Contents | Typical size |
|--------|----------|-------------|
| `FileHeader` | Document header + font table + ALL schematic records | 9 KB - 1.4 MB |
| `Additional` | Header + optional RECORD=225 dashed rectangles | 75 bytes - 3 KB |
| `Storage` | Header + embedded images (compressed) | 6 KB - 270 KB |

Observed across 9 real SchDoc files from a LimeSDR project: all have exactly these 3
streams, 0 storages, 0 read errors.

## Optional streams

The following streams may or may not be present. They were not observed in our test files
but are documented in the .NET loading pipeline:

- **ObjectDefinitions** -- object definition records
- **ReuseBlockInfos** -- reuse block metadata
- **ReuseBlocks** / **ReuseBlocksV2** -- reuse block data (two format versions)
- **HarnessConnectionPointConnector** -- harness connector data
- **Files** -- embedded file data

These are safe to skip if absent. The loading pipeline checks for their existence before
attempting to read them.

## Block encoding format

Every stream uses sequential blocks. Each block begins with a 4-byte little-endian header:

```
bits [23:0]  (lower 24 bits) = payload size in bytes
bits [31:24] (upper 8 bits)  = flags byte
```

Flags values:
- `0x00` -- parameter text block: NUL-terminated pipe-delimited `key=value` pairs encoded
  in Windows-1252. The `RECORD` key identifies the record type.
- `0x01` -- binary data block: raw binary payload. In SchDoc, binary blocks appear **only**
  in the `Storage` stream (embedded images with 0xD0 tag). Unlike SchLib, pins are NOT
  binary blocks in SchDoc.

## Comparison with SchLib CFB structure

| Feature | SchDoc | SchLib |
|---------|--------|--------|
| Top-level storages | 0 | N (one per component + optional alias storages) |
| Streams | 3 always + optional | 3 global + N*10 per component |
| Content organization | Single flat `FileHeader` stream | Per-component `Data` streams |
| Pin format | Text blocks (flags=0x00) in FileHeader | Binary blocks (flags=0x01) in Data |
| Pin sidecar streams | None | Up to 9 per component |
| SectionKeys | Not needed | Required when names > 31 chars |
| Aliases | Not applicable | Alias storages with Redirection |
| Additional stream | Contains RECORD=225 | Not present |

## Stream size observations

From 9 LimeSDR project SchDoc files:

| File | FileHeader | Additional | Storage |
|------|-----------|------------|---------|
| Simple diagrams (01-03) | ~9 KB (65-66 blocks) | 75 bytes (1 block) | 187-269 KB |
| Complex schematics (04-09) | 390 KB - 1.4 MB (1,756-6,447 blocks) | 75 bytes - 2.8 KB | 6-58 KB |

The FileHeader stream size scales linearly with the number of schematic primitives.
Complex schematics with many components and pins can have thousands of blocks.
