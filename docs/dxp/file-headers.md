# FileHeader Streams: Complete Reference

This document describes the `FileHeader` (and related) streams found at the root
of every OLE Compound Binary (CFB) Altium Designer file. Based on inspection of
all files in `data/` using `scripts/ole-inspect.py`.

---

## Overview

Every OLE-based Altium file has a root-level `FileHeader` stream. The format
and content of this stream varies significantly by file type:

| File Type | FileHeader Format | Additional Streams | Encoding |
|-----------|-------------------|-------------------|----------|
| **SchDoc** | Text (pipe-delimited params) | -- | Windows-1252 |
| **SchLib** | Text (pipe-delimited params) | -- | Windows-1252 |
| **PcbDoc** | Binary (legacy v5.0) | `FileHeaderSix` (binary v6.0) | UTF-16LE / ASCII |
| **PcbLib** | Binary (v6.0) | -- | ASCII |
| **PrjPcb** | Not OLE (plain text INI) | -- | UTF-8 w/ BOM |

---

## 1. Schematic Files (SchDoc / SchLib)

### Binary Framing

Both SchDoc and SchLib FileHeader streams use **text block framing**: a sequence
of blocks, each prefixed with a `u32 LE` size followed by that many bytes of
pipe-delimited parameter text:

```
┌─────────────────────────────────────────────┐
│ u32 LE: block_size                          │
├─────────────────────────────────────────────┤
│ payload (block_size bytes)                  │
│   "|KEY1=VALUE1|KEY2=VALUE2|..."            │
└─────────────────────────────────────────────┘
```

Blocks are concatenated end-to-end until the stream is exhausted. Each block
is one parameter record.

### 1.1 SchDoc FileHeader

The SchDoc FileHeader contains **all** schematic primitives for the sheet in a
flat list. The first block is the header record, followed by the sheet
properties record (RECORD=31), template records, and all design objects.

**Stream sizes observed**: 9,288 -- 1,402,002 bytes (scales with design complexity).

#### Block 0: Header Record (no RECORD field)

| Key | Type | Description | Example |
|-----|------|-------------|---------|
| `HEADER` | string | Format identification string | `Protel for Windows - Schematic Capture Binary File Version 5.0` |
| `Weight` | int | Total number of object records that follow | `65`, `5961` |
| `MinorVersion` | int | File minor version | `2` (observed in all samples) |
| `UniqueID` | string | Document unique identifier (8-char alpha) | `LVUUGVHQ` |

#### Block 1: Sheet Properties (RECORD=31)

Contains document-level settings and the **font table**:

| Key | Type | Description |
|-----|------|-------------|
| `RECORD` | int | Always `31` |
| `FontIdCount` | int | Number of font entries (1-based indexing) |
| `Size{N}` | int | Font size in points |
| `Rotation{N}` | int | Font rotation in degrees (0, 90, 180, 270) |
| `Underline{N}` | bool | `T`/`F` |
| `Italic{N}` | bool | `T`/`F` |
| `Bold{N}` | bool | `T`/`F` |
| `StrikeOut{N}` | bool | `T`/`F` |
| `FontName{N}` | string | Font family name |
| `UseMBCS` | bool | Multi-byte character set support |
| `IsBOC` | bool | Binary Object Container flag |
| `HotSpotGridOn` | bool | Hotspot grid enabled |
| `HotSpotGridSize` | int | Hotspot grid size (integer part) |
| `HotSpotGridSize_Frac` | int | Hotspot grid size (fractional part) |
| `SystemFont` | int | System font ID |
| `SheetStyle` | int | Sheet style enum (1=A3, 9=custom, etc.) |
| `BorderOn` | bool | Show border |
| `SheetNumberSpaceSize` | int | Sheet number space size |
| `AreaColor` | int | Background color (decimal BGR) |
| `SnapGridOn` | bool | Snap grid enabled |
| `SnapGridSize` | int | Snap grid size (integer part) |
| `SnapGridSize_Frac` | int | Snap grid size (fractional part) |
| `VisibleGridOn` | bool | Visible grid enabled |
| `VisibleGridSize` | int | Visible grid size (integer part) |
| `VisibleGridSize_Frac` | int | Visible grid size (fractional part) |
| `CustomX` | int | Custom sheet width (in 10-mil units) |
| `CustomY` | int | Custom sheet height (in 10-mil units) |
| `ShowTemplateGraphics` | bool | Show template graphics overlay |
| `TemplateFileName` | string | Path to template file (.SchDot) |
| `Display_Unit` | int | Display units (0=mils, 1=mm) |

#### Blocks 2+: Object Records

Subsequent blocks contain schematic primitives (RECORD=1 through RECORD=241).
The block sequence includes:

- **RECORD=39** (Template): Template reference with `FileName` path
- **RECORD=4** (Label): Text labels from the template
- **RECORD=6** (Polyline): Lines from the template border
- **RECORD=30** (Image): Embedded images (logos, diagrams)
- **RECORD=41** (Parameter): Document parameters (Title, Revision, Date, etc.)
- **RECORD=209** (Note): Annotation notes

All objects reference their parent via `OwnerIndex` (0-based index into the
flat record list). Template objects use `OwnerIndex=1` (owned by the template
record at block 2).

#### Block counts observed:
- Simple sheets (block diagrams): **66 blocks**
- Complex sheets (FPGA, power): **5,962 blocks**

### 1.2 SchLib FileHeader

The SchLib FileHeader is a **single block** containing the library header,
font table, display settings, and a component index.

**Stream sizes observed**: 446 -- 11,876 bytes.

#### Block 0: Library Header + Component Index (single block)

The first section of the record contains the same header fields as SchDoc:

| Key | Type | Description | Example |
|-----|------|-------------|---------|
| `HEADER` | string | Format identification | `Protel for Windows - Schematic Library Editor Binary File Version 5.0` |
| `Weight` | int | Total primitives + aliases count | `5`, `12461` |
| `MinorVersion` | int | File minor version | `2` or `9` |
| `UniqueID` | string | Library unique identifier | `IIEGGIJT` |

Followed by the **font table** (same format as SchDoc RECORD=31) and
**display settings**:

| Key | Type | Description |
|-----|------|-------------|
| `FontIdCount` | int | Number of fonts |
| `Size{N}`, `FontName{N}`, etc. | | Font definitions (same as SchDoc) |
| `UseMBCS` | bool | Always `T` |
| `IsBOC` | bool | Always `T` |
| `SheetStyle` | int | Always `9` (custom) |
| `BorderOn` | bool | Always `T` |
| `SheetNumberSpaceSize` | int | Always `12` |
| `AreaColor` | int | Always `16317695` |
| `SnapGridOn` | bool | Always `T` |
| `SnapGridSize` | int | Snap grid size |
| `VisibleGridOn` | bool | Always `T` |
| `VisibleGridSize` | int | Visible grid size |
| `CustomX` | int | Canvas width |
| `CustomY` | int | Canvas height |
| `UseCustomSheet` | bool | Always `T` |
| `ShowHiddenPins` | bool | Optional (`T` if set) |
| `ReferenceZonesOn` | bool | Always `T` |
| `Display_Unit` | int | Display units (0=mils, 1=mm) |
| `AlwaysShowCD` | bool | Optional |

Then the **component index**:

| Key | Type | Description |
|-----|------|-------------|
| `CompCount` | int | Number of components in library |
| `LibRef{N}` | string | Component reference name (0-based) |
| `CompDescr{N}` | string | Component description (optional) |
| `PartCount{N}` | int | Number of parts (always >= 2: includes "hidden" part 0) |
| `AliasCount{N}` | int | Number of aliases (optional) |
| `Comp{N}Alias{M}` | string | Alias name (0-based) |

#### Observed values:

| File | CompCount | Weight | MinorVersion | FontIdCount |
|------|-----------|--------|--------------|-------------|
| BlankSchlibComponent.SchLib | 1 | 5 | 9 | 1 |
| LimeMicroAltiumLib.SchLib | 200 | 12,461 | 2 | 8 |
| Synthiam.SchLib | 173 | 5,381 | 9 | 4 |

---

## 2. PCB Files (PcbDoc)

PcbDoc files have **two** FileHeader streams: a legacy `FileHeader` (v5.0) and
a modern `FileHeaderSix` (v6.0).

### 2.1 FileHeader (Legacy v5.0)

**Size**: 24 bytes (fixed across all observed files).

Binary structure:

```
Offset  Size  Type     Description
──────  ────  ────     ──────────
0x00    4     u32 LE   Block size (character count = 19)
0x04    19    UTF-16LE Version string (truncated)
0x17    1     u8       Trailing NUL byte
```

The version string is `"PCB 5.0 Binary File"` (19 ASCII characters) encoded in
UTF-16LE, but the `u32` stores the **character count** (19), not the byte count
(38). This means only the first 19 bytes of UTF-16LE data are present --
producing a truncated `"PCB 5.0 B"` followed by an orphan byte `0x69` ('i').

This is a format quirk from the v5.0 era (the length field was originally for
ASCII, but the data was later changed to UTF-16LE without adjusting the
length semantics). Modern Altium reads `FileHeaderSix` instead.

#### Raw hex (both PcbDoc samples):
```
13 00 00 00 50 00 43 00  42 00 20 00 35 00 2e 00
30 00 20 00 42 00 69 00
```

### 2.2 FileHeaderSix (v6.0)

**Size**: 75 bytes (fixed across all observed files).

Binary structure with three fields:

```
Offset  Size     Type         Description
──────  ────     ────         ──────────
0x00    4        u32 LE       Version string block size (N)
0x04    1        u8           Pascal string length (= N)
0x05    N        ASCII        Version string
0x05+N  8        f64 LE       File format version number
0x0D+N  4        u32 LE       UniqueID block size (M)
0x11+N  1        u8           Pascal string length (= M)
0x12+N  M        ASCII        UniqueID string (GUID format)
```

Each string field uses a **redundant pascal block**: the `u32` and `u8` length
values are always equal. Total consumed bytes per block = `4 + 1 + N`.

#### Decoded values:

| Field | Value |
|-------|-------|
| Version string | `"PCB 6.0 Binary File"` |
| Version float | `5.01` |
| UniqueID | GUID string, e.g. `"{C1EF2D32-6634-4C5A-A58E-5AF8D518C64E}"` |

#### Raw hex (LimeSDR_Mini):
```
13 00 00 00 13 50 43 42  20 36 2e 30 20 42 69 6e    PCB 6.0 Bin
61 72 79 20 46 69 6c 65  0a d7 a3 70 3d 0a 14 40    ary File...p=..@
26 00 00 00 26 7b 43 31  45 46 32 44 33 32 2d 36    &...&{C1EF2D32-6
36 33 34 2d 34 43 35 41  2d 41 35 38 45 2d 35 41    634-4C5A-A58E-5A
46 38 44 35 31 38 43 36  34 45 7d                    F8D518C64E}
```

---

## 3. PCB Library Files (PcbLib)

PcbLib files have a **single** `FileHeader` stream using the v6.0 binary format.

### 3.1 FileHeader (v6.0)

**Size**: 53 bytes (fixed across all observed files).

Same binary structure as PcbDoc's `FileHeaderSix`:

```
Offset  Size     Type         Description
──────  ────     ────         ──────────
0x00    4        u32 LE       Version string block size (N=27)
0x04    1        u8           Pascal string length (27)
0x05    27       ASCII        "PCB 6.0 Binary Library File"
0x20    8        f64 LE       5.01
0x28    4        u32 LE       UniqueID block size (M=8)
0x2C    1        u8           Pascal string length (8)
0x2D    8        ASCII        UniqueID (8-char alpha token)
```

#### Decoded values:

| File | Version String | Version Float | UniqueID |
|------|----------------|---------------|----------|
| BlankPcbLibComponent.PcbLib | `PCB 6.0 Binary Library File` | `5.01` | `RTJRBTLE` |
| LimeMicroAltiumLib.PcbLib | `PCB 6.0 Binary Library File` | `5.01` | `YGSAAVRN` |
| Synthiam.PcbLib | `PCB 6.0 Binary Library File` | `5.01` | `XSWCVGPX` |

#### Raw hex (BlankPcbLibComponent):
```
1b 00 00 00 1b 50 43 42  20 36 2e 30 20 42 69 6e    PCB 6.0 Bin
61 72 79 20 4c 69 62 72  61 72 79 20 46 69 6c 65    ary Library File
0a d7 a3 70 3d 0a 14 40  08 00 00 00 08 52 54 4a    ...p=..@.....RTJ
52 42 54 4c 45                                       RBTLE
```

---

## 4. PCB FileVersionInfo Stream

Both PcbDoc and PcbLib files include a `FileVersionInfo/` storage with
`Header` (4-byte u32 = 1) and `Data` streams. The Data stream contains a
single text block (u32 size + pipe-delimited params) recording the version
history of editors that have modified the file.

### Format

```
|COUNT=<N>|VER0=<csv_ascii>|FWDMSG0=<csv_ascii>|BKMSG0=<csv_ascii>|VER1=...|
```

Each version entry has three fields:
- **VER{N}**: Version identifier as comma-separated ASCII values (e.g. `87,105,110,116,101,114,32,48,57` = `"Winter 09"`)
- **FWDMSG{N}**: Forward migration message (displayed when opening in newer versions)
- **BKMSG{N}**: Backward migration message (displayed when opening in older versions, HTML formatted)

#### Observed version entries (BlankPcbLibComponent.PcbLib):

| Index | Version | Backward Message |
|-------|---------|-----------------|
| 0 | `Winter 09` | `<b>CAUTION</b> - Vias support varying diameters across layerstack...` |
| 1 | `Winter 09` | `<b>CAUTION</b> - File may contain pads with hole offsets...` |
| 2 | `Winter 09` | `<b>CAUTION</b> - 3D models now support texturing...` |
| 3 | `Summer 09` | `<b>CAUTION</b> - Support was added for 32 Mechanical Layers...` |
| 4 | `Release 10` | `<b>CAUTION</b> - New Custom Grids...` |

---

## 5. Project Files (PrjPcb)

Project files are **not** OLE/CFB containers. They are plain-text INI-format
files (UTF-8 with BOM). There is no `FileHeader` stream.

Key sections:

| Section | Description |
|---------|-------------|
| `[Design]` | Project settings (hierarchy mode, channel naming, versioning) |
| `[Preferences]` | Vault/revision preferences |
| `[Document{N}]` | Per-document entries with `DocumentPath`, `DocumentUniqueId` |
| `[Configuration{N}]` | Build configurations |

The `DocumentUniqueId` values in the project file match the `UniqueID` values
in the corresponding FileHeader streams (e.g. `IIEGGIJT` for the SchLib,
`RTJRBTLE` for the PcbLib).

---

## 6. UniqueID Cross-Reference

UniqueIDs link documents to their project file entries:

| File | UniqueID | Format |
|------|----------|--------|
| BlankSchlibComponent.SchLib | `IIEGGIJT` | 8-char alpha (SchLib) |
| BlankPcbLibComponent.PcbLib | `RTJRBTLE` | 8-char alpha (PcbLib) |
| LimeMicroAltiumLib.SchLib | `NQSMACOC` | 8-char alpha (SchLib) |
| LimeMicroAltiumLib.PcbLib | `YGSAAVRN` | 8-char alpha (PcbLib) |
| Synthiam.SchLib | `JXVWFLFF` | 8-char alpha (SchLib) |
| Synthiam.PcbLib | `XSWCVGPX` | 8-char alpha (PcbLib) |
| LimeSDR_Mini.PcbDoc | `{C1EF2D32-6634-4C5A-A58E-5AF8D518C64E}` | GUID (PcbDoc) |
| LimeSDR_Mini_panel.PcbDoc | `{EF1DD538-6815-490A-BA8D-0A82481F7E71}` | GUID (PcbDoc) |
| 01_BlockDiagram.SchDoc | `LVUUGVHQ` | 8-char alpha (SchDoc) |
| 02_PowerDiagram.SchDoc | `OJBQGING` | 8-char alpha (SchDoc) |
| 03_ClockDiagram.SchDoc | `RSBYAPWI` | 8-char alpha (SchDoc) |
| 04_LMS7002M_Misc.SchDoc | `WASNWSLX` | 8-char alpha (SchDoc) |
| 05_LMS7002M_RF.SchDoc | `QNDXLOMG` | 8-char alpha (SchDoc) |
| 06_LMS7002M_Power.SchDoc | `QINXVJTH` | 8-char alpha (SchDoc) |
| 07_FPGA.SchDoc | `QVFEWTDW` | 8-char alpha (SchDoc) |
| 08_USB3_0_device.SchDoc | `RRQKEBFM` | 8-char alpha (SchDoc) |
| 09_Misc.SchDoc | `OUOOLFNI` | 8-char alpha (SchDoc) |

Note: Schematic files use 8-character uppercase alpha tokens. PcbDoc files use
standard Windows GUIDs in braces. PcbLib files also use 8-char alpha tokens.

---

## 7. Version String Summary

| File Type | Stream | Version String | Encoding |
|-----------|--------|----------------|----------|
| SchDoc | `FileHeader` block 0 | `Protel for Windows - Schematic Capture Binary File Version 5.0` | Win-1252 text param |
| SchLib | `FileHeader` block 0 | `Protel for Windows - Schematic Library Editor Binary File Version 5.0` | Win-1252 text param |
| PcbDoc | `FileHeader` | `PCB 5.0 Binary File` | UTF-16LE (truncated) |
| PcbDoc | `FileHeaderSix` | `PCB 6.0 Binary File` | ASCII pascal block |
| PcbLib | `FileHeader` | `PCB 6.0 Binary Library File` | ASCII pascal block |

---

## 8. PCB Binary FileHeader Structure (Pseudocode)

```rust
/// Shared structure for PcbLib FileHeader and PcbDoc FileHeaderSix
struct PcbFileHeader {
    // Block 1: version string
    version_block_size: u32,        // = pascal_len
    version_pascal_len: u8,         // = version_block_size
    version_string: [u8; version_pascal_len],  // ASCII

    // Float version
    version_number: f64,            // e.g. 5.01

    // Block 2: unique ID
    uid_block_size: u32,            // = pascal_len
    uid_pascal_len: u8,             // = uid_block_size
    unique_id: [u8; uid_pascal_len],  // ASCII (8-char token or GUID)
}

/// Legacy PcbDoc FileHeader (for format recognition only)
struct PcbFileHeaderLegacy {
    char_count: u32,                // 19 (ASCII character count)
    version_string: [u8; char_count],  // UTF-16LE encoded (truncated to char_count bytes, not char_count*2)
    trailing_nul: u8,               // 0x00
}
```

The pascal block pattern `u32(N) + u8(N) + char[N]` is used consistently:
the u32 and u8 values are always equal and both represent the string length.
Total bytes consumed per pascal block: `5 + N`.
