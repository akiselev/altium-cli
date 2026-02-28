> **Authoritative reference**: See [../../dxp/sidecar-streams-deep-dive.md](../../dxp/sidecar-streams-deep-dive.md)
> for the canonical format specification. This document covers PcbLib-specific details.

# Sidecar Streams

Each footprint storage may contain optional sidecar streams that extend the primitive data
stored in the main `Data` stream. These exist as a backwards-compatibility mechanism: new
data fields were added in separate streams so older readers could still parse the core
binary records.

At load time, sidecar data is merged into primitives by index. At runtime, there is no
distinction between core and sidecar data.

## WideStrings

**Stream**: `<FootprintName>/WideStrings`

### Format (PcbLib-specific — NOT binary TLV!)

PcbLib footprint WideStrings use a **parameter-block format**, completely different from the
binary TLV format used by PcbDoc's `WideStrings6` stream.

```
[4 bytes] u32 LE: block length
[N bytes] NUL-terminated parameter string
```

The parameter string contains comma-separated decimal byte values for each text primitive:

```
|ENCODEDTEXT0=46,68,101,115,105,103,110,97,116,111,114|ENCODEDTEXT1=...|
```

### ENCODEDTEXT decoding

Each `ENCODEDTEXT{N}` value is a comma-separated sequence of decimal byte values representing
a **UTF-8 encoded string**:

```
ENCODEDTEXT4=46,68,101,115,105,103,110,97,116,111,114
             │   │   │   │   │   │   │   │   │   │   │
             .   D   e   s   i   g   n   a   t   o   r
```

Decode by:
1. Split on commas to get decimal byte values
2. Convert to a byte array
3. Decode as UTF-8

The index N corresponds to the text primitive's position in the footprint's primitive list
(only counting Text primitives, type=5).

### Empty WideStrings

A WideStrings stream with only a single byte `0x00` in its block payload indicates no wide
string data (all text fits in the core binary record's Win1252 encoding).

### PcbDoc comparison

PcbDoc's `WideStrings6/Data` uses a binary TLV format with type tags:

| Type | Length | Encoding |
|------|--------|----------|
| `0x06` | 1 byte (u8) | ASCII |
| `0x0C` | 4 bytes (u32 LE) | ASCII |
| `0x12` | 4 bytes (u32 LE, chars) | UTF-16LE |
| `0x14` | 4 bytes (u32 LE) | UTF-8 |

The PcbLib and PcbDoc WideStrings formats share NO structure — they require separate
parser implementations.

## PrimitiveGuids

**Streams**: `<FootprintName>/PrimitiveGuids/Header` + `<FootprintName>/PrimitiveGuids/Data`

### Format

**Header**: 4 bytes — `u32` LE count of GUID entries.

**Data**: The first block contains packed records. The framing is:

```
[4 bytes] u32 LE: block length
[N bytes] packed TPrimitiveGUID records
```

Each `TPrimitiveGUID` record is **NOT a fixed 24-byte struct** in the PcbLib context.
The observed format from Synthiam.PcbLib is:

```
struct PcbLibPrimitiveGuid {
    u32 unknown_zero;        // 4 bytes, always 0x00000000
    u8  guid[16];            // 16 bytes: standard Windows GUID
    u32 primitive_count;     // 4 bytes: number of primitives
    // Then per-primitive entries follow...
}
```

The exact structure needs further investigation with Ghidra/ILSpy against the Delphi code.
The format appears to differ between PcbDoc (24-byte fixed records) and PcbLib.

### Observed data (Synthiam 0402 footprint)

```
Block 0 (85 bytes):
hex: 00 00 00 00 9c 2e 54 78 aa f5 16 43 a9 91 a6 3b 2d 78 6f 75 02 00 00 00 ...
     ─────────── ─────────────────────────────────────────────── ───────────
     zero         GUID (16 bytes)                                 count=2
```

## UniqueIDPrimitiveInformation

**Streams**: `<FootprintName>/UniqueIDPrimitiveInformation/Header` + `.../Data`

### Format

**Header**: 4 bytes — `u32` LE count of entries.

**Data**: Parameter blocks (`u32 LE length + NUL-terminated parameter string`), one per
primitive that has a unique ID assigned.

### Parameter keys

| Key | Example | Description |
|-----|---------|-------------|
| `PRIMITIVEINDEX` | `2` | Zero-based index of the primitive in the footprint |
| `PRIMITIVEOBJECTID` | `Pad` | Object type name (text, not numeric) |
| `UNIQUEID` | `OENFVQGU` | 8-character unique ID string |

### Object type names

The `PRIMITIVEOBJECTID` values are text names, not numeric IDs:

| PRIMITIVEOBJECTID | TObjectId |
|-------------------|-----------|
| `Arc` | 1 |
| `Pad` | 2 |
| `Via` | 3 |
| `Track` | 4 |
| `Text` | 5 |
| `Fill` | 6 |
| `Region` | 11 |
| `ComponentBody` | 12 |

### Notes

- Not every primitive gets a unique ID — simple primitives (tracks, fills) may be skipped.
- The `PRIMITIVEINDEX` is 0-based within the footprint's primitive list (the sequential
  position in the Data stream, counting from the first primitive after the pattern name block).
- Pads almost always have unique IDs; other types vary.

## ExtendedPrimitiveInformation

**Streams**: `<FootprintName>/ExtendedPrimitiveInformation/Header` + `.../Data`

### Format

Same parameter-block format as UniqueIDPrimitiveInformation.

### Parameter keys

| Key | Example | Description |
|-----|---------|-------------|
| `PRIMITIVEINDEX` | `19` | Zero-based primitive index |
| `PRIMITIVEOBJECTID` | `Region` | Object type name |
| `TYPE` | `Mask` | Extended info type |
| `SOLDERMASKEXPANSIONMODE` | `Manual` | Solder mask expansion mode |
| `SOLDERMASKEXPANSION_MANUAL` | `1.9685mil` | Manual solder mask expansion value |
| `PASTEMASKEXPANSIONMODE` | `None` | Paste mask expansion mode |

### Notes

- This stream is **rare** — only 1 footprint (SKY13323-378LF) in the LimeMicro library
  has it, with only 2 entries.
- It provides per-primitive overrides for mask expansion and other properties that were
  added in later format versions.
- The `TYPE` field indicates the kind of extended information (e.g., `Mask` for mask-related
  overrides).

## Sidecar merging process

During load:
1. Parse all primitives from the `Data` stream, assigning each a sequential 0-based index.
2. Read `WideStrings` and merge encoded text into Text primitives by index.
3. Read `UniqueIDPrimitiveInformation` and assign unique IDs to primitives by `PRIMITIVEINDEX`.
4. Read `ExtendedPrimitiveInformation` and merge extended properties by `PRIMITIVEINDEX`.
5. Read `PrimitiveGuids` and assign GUIDs to primitives by their entry mapping.

During save, the reverse process extracts sidecar data from the in-memory primitives and
writes it to the appropriate streams.
