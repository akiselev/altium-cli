# FileHeader Streams

PcbDoc files contain **two** root-level FileHeader streams: a legacy `/FileHeader` (V5 era)
and a modern `/FileHeaderSix` (V6 era). Both are always present. Altium writes both for
backward compatibility; modern Altium reads `FileHeaderSix` and ignores the legacy stream.

## FileHeader (Legacy V5)

The `/FileHeader` stream is a fixed-size legacy artifact from the Protel/DXP V5 era. It
contains only a UTF-16LE-encoded version string with a length quirk.

**Size:** 24 bytes (fixed across all observed files).

### Binary layout

```
Offset  Size  Type      Description
------  ----  ----      -----------
0x00    4     u32 LE    Character count (always 19)
0x04    20    UTF-16LE  Version string bytes (char_count bytes, NOT char_count * 2)
```

### UTF-16LE length quirk

The `u32` at offset 0x00 stores the **character count** of the intended version string
`"PCB 5.0 Binary File"` (19 ASCII characters). However, the stream then contains only
`char_count` raw bytes of UTF-16LE data rather than `char_count * 2` bytes. This truncates
the UTF-16LE encoding partway through:

- Full intended string: `"PCB 5.0 Binary File"` (19 characters, would need 38 bytes in UTF-16LE)
- Actual data: 20 bytes of UTF-16LE = 10 complete code units = `"PCB 5.0 Bi"`

This is a format bug from the V5 era: the length field was designed for single-byte ASCII,
but the payload was later written as UTF-16LE without updating the length semantics. Modern
Altium ignores this stream entirely in favor of `FileHeaderSix`.

### Raw hex (identical in both sample PcbDoc files)

```
13 00 00 00 50 00 43 00  42 00 20 00 35 00 2e 00   ....P.C.B. .5...
30 00 20 00 42 00 69 00                             0. .B.i.
```

Decoded:
- `13 00 00 00` = u32 LE 19 (character count)
- `50 00 43 00 42 00 20 00 35 00 2e 00 30 00 20 00 42 00 69 00` = UTF-16LE `"PCB 5.0 Bi"`

### Parsing

Our Rust parser (`pcb_file_header::parse_pcb_legacy_header`) multiplies the character count
by 2 to get the expected byte count, then decodes as UTF-16LE. For the actual 24-byte
stream, this would expect 38 bytes of UTF-16LE but only 20 are present -- so the parser
reads what is available (the raw `char_count` bytes) and decodes them. The result is the
truncated prefix `"PCB 5.0 Bi"`. This matches how Altium uses it: the legacy header is only
checked for format detection (does it start with `"PCB"` in UTF-16LE?), not for exact string
comparison.

## FileHeaderSix (V6)

The `/FileHeaderSix` stream is the modern file header used by Altium Designer to identify V6
PcbDoc files. It uses the same **pascal-block** binary format as PcbLib's `/FileHeader`.

**Size:** 75 bytes (fixed across all observed files).

### Binary layout

The stream consists of two consecutive pascal-block groups:

```
Offset  Size     Type     Description
------  ----     ----     -----------
--- Block 1: version string + version float ---
0x00    4        u32 LE   String length (N) -- redundant with pascal byte
0x04    1        u8       String length (N) -- pascal length prefix
0x05    N        ASCII    Version string ("PCB 6.0 Binary File", N=19)
0x05+N  8        f64 LE   File format version number (5.01)
--- Block 2: unique ID ---
0x0D+N  4        u32 LE   String length (M) -- redundant with pascal byte
0x11+N  1        u8       String length (M) -- pascal length prefix
0x12+N  M        ASCII    UniqueID string (GUID in braces, M=38)
```

Each string is preceded by both a `u32` and a `u8` that store the same value: the string
byte length. This **redundant pascal block** pattern (`u32(N) + u8(N) + char[N]`) is used
consistently across all PCB binary file headers.

Total bytes: `4 + 1 + N + 8 + 4 + 1 + M = 18 + N + M`

For PcbDoc: N=19 ("PCB 6.0 Binary File"), M=38 (GUID), total = 18 + 19 + 38 = 75 bytes.

### Decoded values

| Field | Value | Notes |
|-------|-------|-------|
| Version string | `"PCB 6.0 Binary File"` | Identifies V6 PcbDoc binary format |
| Version float | `5.01` | f64 LE: `0x40140A3D70A3D70A` |
| UniqueID | GUID in braces | e.g. `"{C1EF2D32-6634-4C5A-A58E-5AF8D518C64E}"` |

### Raw hex (LimeSDR_Mini_1v3_Rounded.PcbDoc)

```
                                                      Block 1: version
13 00 00 00 13 50 43 42  20 36 2e 30 20 42 69 6e   .....PCB 6.0 Bin
61 72 79 20 46 69 6c 65  0a d7 a3 70 3d 0a 14 40   ary File...p=..@
                                                      Block 2: unique ID
26 00 00 00 26 7b 43 31  45 46 32 44 33 32 2d 36   &...&{C1EF2D32-6
36 33 34 2d 34 43 35 41  2d 41 35 38 45 2d 35 41   634-4C5A-A58E-5A
46 38 44 35 31 38 43 36  34 45 7d                   F8D518C64E}
```

Byte-by-byte breakdown:
- `13 00 00 00` = u32 LE 19 (version string length)
- `13` = u8 19 (redundant pascal prefix)
- 19 bytes: `"PCB 6.0 Binary File"`
- `0a d7 a3 70 3d 0a 14 40` = f64 LE `5.01`
- `26 00 00 00` = u32 LE 38 (unique ID string length)
- `26` = u8 38 (redundant pascal prefix)
- 38 bytes: `"{C1EF2D32-6634-4C5A-A58E-5AF8D518C64E}"`

### Raw hex (LimeSDR_Mini_1v3_Rounded_panel.PcbDoc)

```
13 00 00 00 13 50 43 42  20 36 2e 30 20 42 69 6e   .....PCB 6.0 Bin
61 72 79 20 46 69 6c 65  0a d7 a3 70 3d 0a 14 40   ary File...p=..@
26 00 00 00 26 7b 45 46  31 44 44 35 33 38 2d 36   &...&{EF1DD538-6
38 31 35 2d 34 39 30 41  2d 42 41 38 44 2d 30 41   815-490A-BA8D-0A
38 32 34 38 31 46 37 45  37 31 7d                   82481F7E71}
```

Same structure; only the UniqueID differs: `"{EF1DD538-6815-490A-BA8D-0A82481F7E71}"`.

## Version strings

The version string in `FileHeaderSix` identifies the file format variant. Known values from
the C# constants (`xPCBTypes.Consts`):

| Constant | String | Format |
|----------|--------|--------|
| `kCurrentPCBFormat_AD` | `"PCB 6.0 Binary File"` | Altium Designer PcbDoc |
| `kCurrentPCBFormat_CS` | `"CircuitStudio PCB 6.0 Binary File"` | CircuitStudio PcbDoc |
| `kCurrentPCBFormat_CM` | `"CircuitMaker PCB 6.0 Binary File"` | CircuitMaker PcbDoc |
| `kCurrentPCBFormat_PCBWorks` | `"PCBWorks PCB 6.0 Binary File"` | PCBWorks PcbDoc |

For PcbLib, the on-disk version string is `"PCB 6.0 Binary Library File"` (the C# constant
`kCurrentPCBLibFormat = "PCB 6.0 Library File"` is a runtime dispatch string, not the on-disk
value).

## TAdvPCBFileFormatVersion enum

The `RecognizeFile()` method on `IPCB_StructuredStorage` reads the FileHeader streams and
returns one of these format version enum values:

| Value | Name | Description |
|-------|------|-------------|
| 0 | `ePCBFileFormatNone` | Unrecognized |
| 1 | `eAdvPCBFormat_Binary_V3` | Protel 99 SE PcbDoc |
| 2 | `eAdvPCBFormat_Library_V3` | Protel 99 SE PcbLib |
| 3 | `eAdvPCBFormat_ASCII_V3` | Protel 99 SE ASCII |
| 4 | `eAdvPCBFormat_Binary_V4` | DXP PcbDoc |
| 5 | `eAdvPCBFormat_Library_V4` | DXP PcbLib |
| 6 | `eAdvPCBFormat_ASCII_V4` | DXP ASCII |
| 7 | `eAdvPCBFormat_Binary_V5` | Altium Designer PcbDoc (legacy) |
| 8 | `eAdvPCBFormat_Library_V5` | Altium Designer PcbLib (legacy) |
| 9 | `eAdvPCBFormat_ASCII_V5` | Altium Designer ASCII |
| 10 | `eAdvPCBFormat_Binary_V6` | Modern AD PcbDoc (our target) |
| 11 | `eAdvPCBFormat_Library_V6` | Modern AD PcbLib (our target) |
| 12 | `eAdvPCBFormat_ASCII_V6` | Modern ASCII |
| 13 | `eAdvPCBFormat_Binary_V6_CS` | CircuitStudio PcbDoc |
| 14 | `eAdvPCBFormat_Binary_V6_CM` | CircuitMaker PcbDoc |
| 15 | `eAdvPCBFormat_Binary_V6_PCBWorks` | PCBWorks PcbDoc |
| 16 | `eAdvPCBFormat_PadViaLibrary_V6` | Pad/Via library |

## How RecognizeFile works

The `RecognizeFile()` method (declared on `IPCB_StructuredStorage`) is implemented in the
Delphi side and performs format detection by reading the header streams. The process:

1. Opens the CFB container
2. Checks for `/FileHeaderSix` stream
   - If present: reads the pascal-block version string and matches against known V6 format
     strings (`"PCB 6.0 Binary File"`, `"CircuitStudio PCB 6.0 Binary File"`, etc.)
   - Also reads the `f64` version number and returns it via the `ref double argVersion`
     overload
3. If no `FileHeaderSix` or no match: falls back to `/FileHeader`
   - Reads the UTF-16LE legacy header and matches against V5/V4/V3 format patterns
4. Returns the appropriate `TAdvPCBFileFormatVersion` enum value

The two-overload pattern (`RecognizeFile()` and `RecognizeFile(ref double argVersion)`)
allows callers to optionally receive the f64 version number from the FileHeaderSix block.

## UniqueID format

PcbDoc files use **Windows GUIDs in braces** as UniqueIDs (e.g.,
`"{C1EF2D32-6634-4C5A-A58E-5AF8D518C64E}"`). This differs from PcbLib files which use
**8-character uppercase alpha tokens** (e.g., `"RTJRBTLE"`).

The UniqueID in `FileHeaderSix` matches the `DocumentUniqueId` in the parent `.PrjPcb`
project file, linking the document to its project entry.

### Observed UniqueIDs

| File | UniqueID |
|------|----------|
| LimeSDR_Mini_1v3_Rounded.PcbDoc | `{C1EF2D32-6634-4C5A-A58E-5AF8D518C64E}` |
| LimeSDR_Mini_1v3_Rounded_panel.PcbDoc | `{EF1DD538-6815-490A-BA8D-0A82481F7E71}` |

## Differences from PcbLib FileHeader

PcbDoc and PcbLib use the same pascal-block binary format for their V6 headers, but differ in:

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| Legacy stream | Has `/FileHeader` (UTF-16LE V5) | No legacy stream |
| V6 stream name | `/FileHeaderSix` | `/FileHeader` |
| Version string | `"PCB 6.0 Binary File"` | `"PCB 6.0 Binary Library File"` |
| UniqueID format | GUID in braces (38 chars) | 8-char uppercase alpha token |
| Stream size | 75 bytes | 53 bytes |
| Legacy stream size | 24 bytes | N/A |

PcbDoc has both streams because it evolved from V5 (which used the legacy format) to V6
(which added `FileHeaderSix`). PcbLib was created in the V6 era and only has a single
`/FileHeader` using the modern pascal-block format.

## Rust implementation

The parser lives in `crates/altium-format/src/pcb_file_header.rs`:

- `parse_pcb_file_header(data)` -- parses the pascal-block format (FileHeaderSix / PcbLib FileHeader)
- `parse_pcb_legacy_header(data)` -- parses the legacy UTF-16LE format (PcbDoc FileHeader)

**Note:** The current parser's `parse_pcb_file_header` treats the leading `u32` as an outer
block length for a sub-reader, which would differ from the actual on-disk format where the
`u32` equals the string length (not a full block size). The parser's unit tests build
synthetic data where `u32 = 1 + string_len + 8` (total block content), but real files have
`u32 = string_len`. This will need to be reconciled when integrating with the real PcbDoc
loader.
