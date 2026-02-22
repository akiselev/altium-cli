# CFB Structure

PcbLib files use the OLE Compound Binary (CFB / Structured Storage) format. The storage
tree looks like this:

```
Root Storage
 |
 +-- FileHeader                       (stream: library format identification)
 +-- SectionKeys                      (stream: optional, footprint name-to-key mapping)
 |
 +-- FileVersionInfo/                 (storage: file version history)
 |    +-- Header                      (stream: u32 count)
 |    +-- Data                        (stream: version info parameter blocks)
 |
 +-- Library/                         (storage: library-wide global data)
 |    +-- Header                      (stream: u32 count)
 |    +-- Data                        (stream: library board defaults, layer stack)
 |    +-- EmbeddedFonts               (stream: embedded font binary data)
 |    +-- ComponentParamsTOC/
 |    |    +-- Header                 (stream: u32 count)
 |    |    +-- Data                   (stream: component parameter table of contents)
 |    +-- LayerKindMapping/
 |    |    +-- Header                 (stream: u32 count)
 |    |    +-- Data                   (stream: mechanical layer kind mapping)
 |    +-- Models/
 |    |    +-- Header                 (stream: u32 count of model entries)
 |    |    +-- Data                   (stream: model metadata parameter blocks)
 |    |    +-- 0                      (stream: zlib-compressed STEP model data)
 |    |    +-- 1                      (stream: zlib-compressed STEP model data)
 |    |    +-- ...                    (one stream per embedded 3D model)
 |    +-- ModelsNoEmbed/
 |    |    +-- Header                 (stream: u32 count)
 |    |    +-- Data                   (stream: references to non-embedded models)
 |    +-- PadViaLibrary/
 |    |    +-- Header                 (stream: u32 count)
 |    |    +-- Data                   (stream: pad/via template library parameters)
 |    +-- Textures/
 |         +-- Header                 (stream: u32 count)
 |         +-- Data                   (stream: texture image data)
 |
 +-- <FootprintName>/                 (one storage per footprint)
 |    +-- Data                        (stream: pattern name + packed binary primitives)
 |    +-- Header                      (stream: u32 record count)
 |    +-- Parameters                  (stream: footprint metadata key=value block)
 |    +-- WideStrings                 (stream: parameter-block format, NOT binary TLV)
 |    +-- PrimitiveGuids/
 |    |    +-- Header                 (stream: u32 count)
 |    |    +-- Data                   (stream: binary GUID table, 24 bytes/entry)
 |    +-- UniqueIDPrimitiveInformation/
 |    |    +-- Header                 (stream: u32 count)
 |    |    +-- Data                   (stream: per-primitive unique IDs, param blocks)
 |    +-- ExtendedPrimitiveInformation/  (rare, not present in all libraries)
 |         +-- Header                 (stream: u32 count)
 |         +-- Data                   (stream: per-primitive extended properties)
 |
 +-- <AnotherFootprint>/
      +-- ...
```

## Stream inventory from real files

Verified against three PcbLib files from our test corpus:

| Stream | BlankPcbLib | LimeMicro (281 footprints) | Synthiam (482 footprints) |
|--------|:-----------:|:---------:|:--------:|
| FileHeader | Yes | Yes | Yes |
| SectionKeys | No | No | Yes (2 entries) |
| FileVersionInfo/{Header,Data} | Yes | No | No |
| Library/{Header,Data} | Yes | Yes | Yes |
| Library/EmbeddedFonts | Yes | Yes | Yes |
| Library/ComponentParamsTOC/{H,D} | Yes | Yes | Yes |
| Library/LayerKindMapping/{H,D} | Yes | Yes | Yes |
| Library/Models/{H,D,0..N} | Yes (empty) | Yes (121 models) | Yes (empty Data) |
| Library/ModelsNoEmbed/{H,D} | Yes | Yes | Yes |
| Library/PadViaLibrary/{H,D} | Yes | Yes | Yes |
| Library/Textures/{H,D} | Yes | Yes | Yes |
| Per-footprint Data | Yes | Yes | Yes |
| Per-footprint Header | Yes | Yes | Yes |
| Per-footprint Parameters | Yes | Yes | Yes |
| Per-footprint WideStrings | No | Yes | Yes |
| Per-footprint PrimitiveGuids/{H,D} | Yes | No | Yes |
| Per-footprint UniqueIDPrimitiveInformation/{H,D} | No | Yes (276 of 281) | Yes (476 of 482) |
| Per-footprint ExtendedPrimitiveInformation/{H,D} | No | Yes (1 footprint only) | No |

**Key observations:**
- `FileVersionInfo` is only present in blank/newly-created libraries
- `SectionKeys` only appears when footprint names exceed 31 characters
- `WideStrings` is absent from newly-created blank footprints
- `PrimitiveGuids` and `UniqueIDPrimitiveInformation` are mutually exclusive in some files (LimeMicro has UniqueID but not PrimitiveGuids; the blank lib has PrimitiveGuids but not UniqueID) — newer format versions may prefer one over the other
- `ExtendedPrimitiveInformation` is rare — only 1 footprint in LimeMicro has it
- Not all footprints have UniqueID or PrimitiveGuids — simple footprints (like arrows/etch marks in Synthiam) may lack them

## Header/Data stream pattern

Most sub-storages in PcbLib follow the same **Header + Data** pattern:

- **Header**: Always exactly 4 bytes — a `u32` little-endian record count
- **Data**: Variable-length payload whose format depends on the section type

This pattern is shared with PcbDoc and should use the same section-reading infrastructure.

## Footprint storage key rules

CFB storage names are limited to 31 characters. The mapping from footprint name to CFB
storage key follows these rules (identical to SchLib):

1. Names longer than 31 characters are truncated to 31 characters.
2. If truncation produces a collision, an unspecified disambiguation scheme is used.
3. When any footprint name exceeds 31 characters, the `/SectionKeys` stream provides
   the full name-to-key mapping.
4. For names <= 31 characters, the name itself is used directly as the CFB storage key.

## System storages

These top-level storages are NOT footprints and must be excluded when enumerating footprints:

- `FileVersionInfo`
- `Library`

The `SectionKeys` stream is at the root level (not a storage) and the `FileHeader` stream
is also at the root level.
