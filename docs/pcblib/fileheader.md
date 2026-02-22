# FileHeader Stream

The `/FileHeader` stream at the CFB root identifies the library format version.

## Binary layout

```
[4 bytes] u32 LE: block length (total size of remaining block content)
[1 byte]  u8: header text string length
[N bytes] ASCII header text (e.g., "PCB 6.0 Binary Library File")
[8 bytes] f64 LE: file format version float (unused for validation, informational only)
[4 bytes] u32 LE: key block length
[1 byte]  u8: key string length
[N bytes] ASCII key token (e.g., "RTJRBTLE")
```

## Example (from BlankPcbLibComponent.PcbLib)

Raw hex: `1b 00 00 00 1b 50 43 42 20 36 2e 30 20 42 69 6e 61 72 79 20 4c 69 62 72 61 72 79 20 46 69 6c 65 ...`

Decoded:
- Block length: `0x0000001b` = 27
- Header text length: 27
- Header text: `"PCB 6.0 Binary Library File"`
- Version float: `5.01` (f64 LE)
- Key block length: 8
- Key text length: 8
- Key token: `"RTJRBTLE"`

## Format identification

The header text string identifies the file format:

| Header text | Format |
|-------------|--------|
| `"PCB 6.0 Binary Library File"` | Modern PcbLib (V6) — the format we support |
| `"PCB 5.0 Binary File"` | PcbDoc (NOT a library) |

The `TAdvPCBFileFormatVersion` enum values relevant to PcbLib:

| Value | Name | Description |
|-------|------|-------------|
| 2 | `eAdvPCBFormat_Library_V3` | Protel 99 SE library |
| 5 | `eAdvPCBFormat_Library_V4` | DXP library |
| 8 | `eAdvPCBFormat_Library_V5` | Altium Designer library |
| 11 | `eAdvPCBFormat_Library_V6` | Modern AD library (our target) |

## Key token

The key token (e.g., `"RTJRBTLE"`) is an 8-character string used by Altium internally.
Its purpose is unclear but it appears consistent across files of the same format. For
parsing purposes, we read it but do not validate against a specific value.

## Comparison with SchLib FileHeader

SchLib's `/FileHeader` is completely different — it contains the full font table, component
index, and library display parameters as a pipe-delimited parameter block. PcbLib's
FileHeader is purely a binary format identifier with no library metadata.

All library-wide metadata in PcbLib is instead stored in the `/Library/Data` stream.
