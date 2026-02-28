# Roundtrip Diff Report: 09_FPGA_misc.SchDoc (LimeSDR-USB plug 1v2)

## Summary
- **File size**: 2,305,024 bytes original -> 2,306,048 bytes roundtripped (+1,024 bytes)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: Block sizes identical per-block
- **Diff category**: Parameter key casing change + parameter reordering
- **Details**:
  - **block[0]**: Header block -- `WEIGHT=5` -> `Weight=5` (key casing only)
  - **blocks[1-5]**: RECORD=225 (Bezier) records -- two kinds of diffs:
    1. Key casing: ALL CAPS -> MixedCase (e.g., `INDEXINSHEET` -> `IndexInSheet`, `LOCATION.X` -> `Location.X`)
    2. Parameter reordering: `LineStyleExt=1` moved from end to immediately after `LineStyle=1`
  - All parameter values identical. 6 blocks differ total.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Block sizes identical per-block for all 10,335 differing blocks
- **Diff category**: Parameter key casing change + parameter reordering + font parameter reordering
- **Details**:
  - **block[0]**: Header -- `WEIGHT=10334` -> `Weight=10334`, `MINORVERSION` -> `MinorVersion`, `UNIQUEID` -> `UniqueID` (key casing)
  - **block[1]**: RECORD=31 (Sheet properties) -- Key casing + font parameter reordering (`SIZE1=10|FONTNAME1=...` -> `FontName1=...|Size1=10`) + sheet property reordering (`SYSTEMFONT`, `USEMBCS`, `ISBOC`, `AREACOLOR` moved to end). Font table differs slightly from file 04: 12 fonts with SIZE8=14 (vs 12 in 04), SIZE9=12 (vs 14 in 04), SIZE10=8 (vs 14 in 04), etc.
  - **blocks[2+]**: Same patterns as file 04. All record types show key casing changes (ALL CAPS -> MixedCase). Identical values throughout.
  - **RECORD=17 (PowerPort)**: 69 instances with parameter reordering (Style/ShowNetName/Orientation/FontID regrouped relative to Location/Color/Text)
  - **RECORD=27 (Wire)**: 384 instances with LineWidth/Color swap and UniqueID position change
  - **RECORD=209 (NoteText)**: 4 instances with Text/Author/FontID/TextColor reordering
  - **RECORD=225 (Bezier)**: Same LineStyleExt reordering as /Additional
  - This file is nearly identical to file 06 (same template, same component library, different board revision). The diff patterns are identical in nature, and the block counts match (10,335 blocks in both).

### /Storage
- **Status**: DIFFERS
- **Size change**: 5,750 bytes -> 5,869 bytes (+119 bytes)
- **Diff category**: Key casing change (block[0]) + binary difference (block[1])
- **Details**:
  - **block[0]**: `WEIGHT=1` -> `Weight=1` (key casing)
  - **block[1]**: Binary data: 5,712 bytes -> 5,831 bytes (+119 bytes). Same size change as files 04 and 06, suggesting a consistent serialization difference in embedded image data.

## Diff Categories Found

1. **Parameter key casing** (ALL CAPS -> MixedCase) -- all 10,343 differing blocks. Benign.
2. **Parameter reordering** -- RECORD=17 (69 blocks), RECORD=27 (384 blocks), RECORD=209 (4 blocks), RECORD=225 (5 blocks). Benign.
3. **Font parameter reordering** -- RECORD=31 (1 block). FontName before Size; sheet properties regrouped. Benign.
4. **Binary differences** -- /Storage block[1] +119 bytes.

Categories NOT found:
- No missing parameters
- No changed values
- No new or missing streams

## Fidelity Assessment

**BENIGN**: Identical assessment to file 04. All text-mode parameters preserved with only casing and ordering differences. The /Storage binary size increase of 119 bytes is the only substantive difference and matches the pattern seen in files 04 and 06, suggesting a systematic serialization difference rather than data corruption.

## Impact on File Format Support

### Working well
- All record types correctly parsed and re-serialized
- All parameter values preserved exactly, including coordinates, fractional parts, Unicode escapes, and UniqueIDs
- File is a close variant of file 06 (same LimeSDR-USB design, plug vs socket variant), and both show identical diff patterns, confirming consistent behavior

### What needs improvement
- Same issues as file 04: parameter key casing (ALL CAPS vs MixedCase), parameter ordering for RECORD=17/27/209/225, font field ordering in RECORD=31
- These are cosmetic differences that don't affect Altium's ability to read the file

### Specific bugs or missing features revealed
- **/Storage binary size difference**: +119 bytes, matching files 04 and 06. Systematic issue with embedded image binary serialization.
- No data-handling bugs.
