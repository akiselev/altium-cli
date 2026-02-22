# Footprint Data Stream

Each footprint's `<FootprintName>/Data` stream contains the footprint's graphical primitives
in packed binary format. This is the core data that defines the footprint's pads, tracks,
text labels, regions, and 3D body references.

## Stream layout

```
┌─────────────────────────────────────────────────────────┐
│ Pattern Name Block                                       │
│   [4 bytes] u32 LE: block length (string_len + 1)       │
│   [1 byte]  u8: pattern name string length               │
│   [N bytes] ASCII pattern name                           │
├─────────────────────────────────────────────────────────┤
│ Primitive Record 0                                       │
│   [1 byte]  u8: TObjectId type byte                      │
│   [4 bytes] u32 LE: record length (flags in high byte)   │
│   [N bytes] binary payload                               │
├─────────────────────────────────────────────────────────┤
│ Primitive Record 1                                       │
│   ...                                                    │
├─────────────────────────────────────────────────────────┤
│ ...                                                      │
└─────────────────────────────────────────────────────────┘
```

## Pattern name block

The first block in the Data stream identifies the footprint by name:

```
[4 bytes] u32 LE: total block length
[1 byte]  u8: string length (N)
[N bytes] ASCII pattern name (e.g., "CAP0402")
```

The block length equals `string_length + 1` (the 1 accounts for the string length byte).

Example (CAP0402):
```
hex: 08 00 00 00 07 43 41 50 30 34 30 32
     ─────────── ── ───────────────────────
     block_len=8  7  "CAP0402"
```

The pattern name should match the `PATTERN` value in the Parameters stream and the CFB
storage name (or the full name from SectionKeys if truncated).

## Primitive record framing

After the pattern name block, primitive records are packed consecutively:

```
[1 byte]  TObjectId type byte (1=Arc, 2=Pad, 3=Via, 4=Track, 5=Text, 6=Fill, 11=Region, 12=ComponentBody)
[4 bytes] u32 LE record length
[N bytes] record payload (binary struct, size = record_length)
```

### Record length field

The u32 record length may have **flags in the high byte**. Apply the size mask to extract
the actual payload length:

```
actual_length = raw_u32 & 0x00FFFFFF    // lower 24 bits = size
flags = (raw_u32 >> 24) & 0xFF          // upper 8 bits = flags
```

In practice, flags are usually 0x00 for standard records. The flag byte usage in PcbLib
Data streams appears minimal compared to PcbDoc.

### Multi-subrecord types

Some object types have multiple subrecords per primitive:

| Object Type | TObjectId | Subrecord Count | Notes |
|-------------|-----------|----------------|-------|
| Pad | 2 | 6 | Main body + 5 additional shape/parameter subrecords |
| Text | 5 | 2 | Main body + text string subrecord |
| All others | 1,3,4,6,11,12 | 1 | Single subrecord per primitive |

For multi-subrecord types, each subrecord has its own `[4 bytes] u32 LE length + [N bytes] payload`
framing. The type byte only appears once at the beginning — the subsequent subrecords are
implicitly part of the same primitive.

Example (Pad with 6 subrecords):
```
[u8 type=2]          // Pad object ID
[u32 len][payload]   // Subrecord 0: main pad data
[u32 len][payload]   // Subrecord 1: additional data
[u32 len][payload]   // Subrecord 2: additional data
[u32 len][payload]   // Subrecord 3: additional data
[u32 len][payload]   // Subrecord 4: additional data
[u32 len][payload]   // Subrecord 5: additional data
```

Example (Text with 2 subrecords):
```
[u8 type=5]          // Text object ID
[u32 len][payload]   // Subrecord 0: text properties
[u32 len][payload]   // Subrecord 1: text string content
```

## Header stream

The `<FootprintName>/Header` stream is always 4 bytes: a `u32` LE value. In the context
of PcbLib footprints, the header appears to contain the primitive count (number of records
in the Data stream), though for empty footprints it may be 0.

Example (blank footprint): `00 00 00 00` → count = 0

## Object types observed in test files

From scanning our test corpus (LimeMicro: 281 footprints, Synthiam: 482 footprints):

| TObjectId | Name | LimeMicro Count | Synthiam Count | Description |
|-----------|------|:-------:|:-------:|-------------|
| 1 | Arc | 324 | 231 | Circular arcs (courtyard, silkscreen) |
| 2 | Pad | 9,123 | 5,109 | Component pads (most common) |
| 3 | Via | 441 | 0 | Vias (rare in libraries) |
| 4 | Track | 4,086 | 2,554 | Line segments (silkscreen outlines) |
| 5 | Text | 411 | 66 | Text strings (designator, comment) |
| 6 | Fill | 17 | 115 | Solid fills |
| 11 | Region | 474 | 0 | Regions (courtyard, copper shapes) |
| 12 | ComponentBody | 886 | 42 | 3D body references |

**Not observed** in PcbLib footprints (PcbDoc-only): Connection(7), Net(8), Component(9),
Polygon(10), Dimension(13), Coordinate(14).

## Typical footprint structure

A simple SMD footprint (e.g., CAP0402) contains:
1. Pattern name block: `"CAP0402"`
2. Pad records (type=2): SMD pads for each pin
3. Track records (type=4): Silkscreen outline on overlay layer
4. Text record (type=5): `.Designator` text
5. Region record (type=11): Courtyard area
6. ComponentBody record (type=12): 3D model reference

A complex BGA footprint (e.g., 10M16SAU169C8G) contains:
1. Pattern name block
2. Arc records (type=1): Courtyard arcs
3. 169 Pad records (type=2): BGA ball pads
4. Track records (type=4): Silkscreen
5. Text record (type=5): `.Designator`
6. Region record (type=11): Courtyard
7. ComponentBody records (type=12): 3D model reference(s)
