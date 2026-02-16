# Destructive Round-Trip Test Results

Results from running destructive round-trip tests against Synthiam.SchLib and Synthiam.PcbLib.
These tests read the original file, rebuild from scratch using v2 API (cloning RecordOrigins,
marking dirty to force re-serialization), save to new CFB, and compare at stream level.

## SchLib Results (Synthiam.SchLib)

- **172 components** — count matches perfectly
- **174 differences** found (107 text diffs, 65 binary diffs, 2 streams only in original)

### SchLib Issues

| # | Category | Count | Root Cause |
|---|----------|-------|------------|
| 1 | **FileHeader incomplete** | 1 | `write_file_header()` only writes HEADER, Weight, MinorVersion, UniqueID, CompCount, LibRef/CompDescr/PartCount per component. The original has many more fields: ALIASCOUNT, AREACOLOR, BORDERON, COMP*ALIAS*, font info, etc. |
| 2 | **Data stream text diffs** | 107 | Data streams compared as flat text param strings, but they're length-prefixed record sequences. Diff shows trailing bytes after values (e.g. `PARTIDLOCKED: "F\0x\0\0\0" -> "F\0m\0\0\0"`), suggesting records are being conflated during comparison, OR the re-serialization changes trailing binary data after text records. |
| 3 | **Data stream binary diffs** | 65 | Some Data streams have length differences (e.g. orig 1313 bytes, rebuilt 1298 bytes). Re-serialization via `to_param_string()` may produce different output than original (param ordering, missing fields, size prefix differences). |
| 4 | **Missing streams** | 2 | `/PIC10F220/Redirection` and `/Storage` streams exist in original but are not written by v2 save path. |

## PcbLib Results (Synthiam.PcbLib)

- **482 footprints** — count matches perfectly
- **1425 differences** found (0 text diffs, 932 binary diffs, 492 only in original, 513 matched)

### PcbLib Issues

| # | Category | Count | Root Cause |
|---|----------|-------|------------|
| 1 | **Primitives writing too small** | ~480 | Data streams are much smaller in rebuilt (e.g. orig 518 bytes, rebuilt 22 bytes). When dirty, binary primitives appear to produce minimal output. Need to investigate whether `raw_block` loses data during clone+dirty cycle. |
| 2 | **Missing streams** | 492 | Many storages in original not written by PcbLib save. Could be `Library` top-level storage, section keys, or other metadata storages. |
| 3 | **Header diffs** | ~450 | Header streams (4 bytes each) differ between original and rebuilt. These contain primitive counts — if primitives aren't writing correctly, counts will differ. |
| 4 | **Parameter streams** | 0 diffs | All footprint metadata (pipe-delimited params) round-trip perfectly. |

## Root Cause Analysis

### PcbLib Issue #1: Multi-Subrecord Primitive Parsing (CRITICAL)

**Root cause found.** The `parse_pcb_data_stream()` function in `v2/documents/pcblib.rs:499-551`
assumes ALL primitives use a simple `type(1) + len(4) + data(len)` framing — one subrecord per
primitive. This is WRONG for Pad and Text, which have multiple subrecords.

**Per `docs/notes/altium-NOTES.md` (from Ghidra decompilation):**

| Type ID | Name | Subrecord Count | Framing |
|---------|------|-----------------|---------|
| 1 | Arc | 1 | `type(1) + u32 len + data` |
| 2 | **Pad** | **6** | `type(1) + 6×(u32 len + data)` |
| 3 | Via | 1 | `type(1) + u32 len + data` |
| 4 | Track | 1 | `type(1) + u32 len + data` |
| 5 | **Text** | **2** | `type(1) + 2×(u32 len + data)` |
| 6 | Fill | 1 | `type(1) + u32 len + data` |
| 11 | Region | 1 | `type(1) + u32 len + data` |
| 12 | ComponentBody | 1 | `type(1) + u32 len + data` |

**What happens:** For the "H" footprint Data stream (3207 bytes):

1. Pattern name: `u32 len=1 + "H"` = 5 bytes
2. Arc[0]: `type=1 + len=56 + data(56)` = 61 bytes — **parsed correctly**
3. Arc[1]: `type=1 + len=56 + data(56)` = 61 bytes — **parsed correctly**
4. Pad: `type=2 + [subrecord1: u32 len=2 + data(2)]` — parser reads ONLY the first subrecord
   (pad name, 2 bytes: `0x01 0x31`) and thinks the pad is done.
5. The remaining 5 pad subrecords (~910 bytes) are misinterpreted as new primitives:
   - Subrecord 2's `u32 length` bytes get read as `type + len` of a phantom primitive
   - This cascades through all remaining subrecords
   - Eventually hits a byte sequence that decodes as a huge length → `block_len > data.len()` → parser breaks
6. **Result:** Only 4 primitives captured (2 arcs + truncated pad + phantom), 3067 of 3207 bytes unparsed

**The existing `parse_pad()` in `pcb_pad.rs:63` already knows about 6 subrecords** — it walks
through 4 string subrecords to find subrecord 5. But it never gets called properly because
`parse_pcb_data_stream()` only feeds it the first subrecord's 2 bytes.

**Fix:** `parse_pcb_data_stream()` needs a subrecord count lookup per type ID:
- After reading the type byte, determine how many subrecords to read
- For multi-subrecord types (Pad=6, Text=2), read all `(u32 len + data)` blocks
- Store all subrecords (with their length prefixes) in the `raw_block`
- `build_pcb_data_stream()` needs the same logic for writing

**Pad 6 subrecords (from Ghidra FUN_0187eb60):**
1. Pad Name (WxString, e.g. "1", "A1")
2. Unknown string (often empty, length=1 with null byte)
3. Unknown string (often `|&|0`, length=5)
4. Unknown string (often empty)
5. Main Pad Data (172 bytes in AD26, minimum 110)
6. Per-Layer Stack Data (596/628/651 bytes)

**Text 2 subrecords:**
1. Main text data (252 bytes in AD26, minimum 40)
2. Text string (variable length, null-terminated ASCII)

### PcbLib Issue #2: Missing Streams (492 streams)

**Root cause:** PcbLib save only writes `Header`, `Data`, and `Parameters` per footprint.
The original file has additional per-footprint and library-level streams.

**Per-footprint streams not written:**
- `WideStrings` — UTF-16 encoded text strings (referenced by `widestring_index` in Text records)
- `PrimitiveGuids` — GUID mapping for primitives
- `UniqueIDPrimitiveInformation` — Unique ID tracking

**Library-level streams not written:**
- `Library/EmbeddedFonts`
- `Library/ComponentParamsTOC/`
- `Library/LayerKindMapping/`
- `Library/Models/` — Embedded 3D models (zlib-compressed STEP files)
- `Library/ModelsNoEmbed/` — External model references
- `Library/PadViaLibrary/` — Shared pad/via template definitions
- `Library/Textures/`
- `SectionKeys` — Name → storage key mapping

**Impact:** 492 missing streams = ~482 footprints × (WideStrings + PrimitiveGuids + UniqueID)
≈ 482 × ~1 stream average, plus library-level streams.

### PcbLib Issue #3: Header Diffs (~450)

**Root cause:** Cascading from Issue #1. The Header stream contains a `u32` primitive count.
Because our parser drops most primitives (only captures the first subrecord of Pads/Texts),
the rebuilt primitive count is wrong → Header value is wrong → binary diff.

**Fix:** Resolving Issue #1 will automatically fix most Header diffs.

### SchLib Issue #1: FileHeader Incomplete

**From AD26 decompiled code (SchDataExporterLibraryV5.cs):**

The original FileHeader contains many fields our `write_file_header()` doesn't write:

- **Font table**: `FontIdCount`, then per-font: `Size{N}`, `Rotation{N}`, `Underline{N}`,
  `Italic{N}`, `Bold{N}`, `StrikeOut{N}`, `FontName{N}`
- **Library properties**: `Description`, `BorderOn`, `AreaColor`, `SnapGridOn`, `SnapGridSize`,
  `VisibleGridOn`, `VisibleGridSize`, `CustomX`, `CustomY`, `UseCustomSheet`
- **Per-component aliases**: `AliasCount{N}`, `CompAlias{N}_{M}`
- **Vault info**: `ReleaseVaultGUID`, `FolderGUID`, `LifeCycleDefinitionGUID`,
  `RevisionNamingSchemeGUID`, `LifeCycleStatusGUID`

**Fix:** Either preserve the raw FileHeader bytes (non-destructive), or extend
`write_file_header()` to include all known fields.

### SchLib Issue #2: Data Stream Record Diffs (107 text + 65 binary)

**Likely causes:**
1. **Comparison artifact**: Data streams are length-prefixed record sequences, but our
   CFB comparison treats them as flat text. The "diff" may show trailing bytes from one
   record bleeding into the next record's display.
2. **Param re-serialization changes**: `ParameterCollection::to_param_string()` may emit
   parameters in a different order than the original. While semantically equivalent, this
   changes the binary content.
3. **Missing fields**: Some record types may have fields that `#[altium(skip)]` drops
   (e.g., polyline/polygon vertices). Re-serialization produces shorter records.

**Fix:** Improve comparison to parse SchLib Data streams record-by-record (respecting
the length-prefixed format with `mode >> 24` flag). Compare each record individually.

**SchLib Data stream record format (from SchDataSerializerParam.cs):**
```
For each record:
  u32: length | (mode << 24)
  mode = value >> 24      (0x00=ASCII text, 0x01=binary)
  actual_length = value & 0x00FFFFFF
  [actual_length bytes]: record data
```

### SchLib Issue #3: Missing Streams (2)

- `/PIC10F220/Redirection` — Maps component aliases to actual components
- `/Storage` — Contains embedded image/binary data (SchDataEmbeddedObject)

**Additional per-component streams not written:**
- `PinFrac`, `PinDesc`, `PinMiscData`, `PinTextData`, `PinWideText`

## Priority Fix Order

1. **PcbLib multi-subrecord parsing** — Fix `parse_pcb_data_stream()` to handle Pad (6) and
   Text (2) subrecord counts. This fixes ~1400 of the 1425 PcbLib differences (Issues #1, #3).
2. **PcbLib missing streams** — Add WideStrings, PrimitiveGuids, UniqueID per footprint + library streams.
3. **SchLib FileHeader** — Extend `write_file_header()` or preserve raw bytes.
4. **SchLib Data comparison** — Improve test to compare record-by-record.
5. **SchLib missing streams** — Add Storage, Redirection, pin-related streams.
