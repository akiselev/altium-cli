# Roundtrip Diff Report: 09_FPGA_misc.SchDoc (LimeSDR-USB socket 1v2)

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
  - **blocks[1-5]**: RECORD=225 (Bezier) records -- key casing change (ALL CAPS -> MixedCase) + `LineStyleExt=1` moved from end to after `LineStyle=1`
  - All parameter values identical. 6 blocks differ total.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Block sizes identical per-block for all 10,335 differing blocks
- **Diff category**: Parameter key casing change + parameter reordering + font parameter reordering
- **Details**:
  - This file produces diffs that are functionally identical to file 05 (LimeSDR-USB plug 1v2). Both files:
    - Have 10,335 differing blocks in /FileHeader
    - Share the same template (A3_LMS.SchDot)
    - Have the same font table (12 fonts, same sizes/attributes)
    - Contain the same record types with the same diff patterns
  - **block[0]**: Header -- `WEIGHT=10334` -> `Weight=10334` (key casing; note: same WEIGHT value as file 05)
  - **block[1]**: RECORD=31 -- Font parameter reordering + key casing + sheet property reordering (identical pattern to file 05)
  - **blocks[2+]**: Key casing changes throughout. Same record types and diff patterns as file 05.
  - **RECORD=17 (PowerPort)**: 69 instances, parameter reordering
  - **RECORD=27 (Wire)**: 384 instances, LineWidth/Color swap + UniqueID position
  - **RECORD=209 (NoteText)**: 4 instances, Text/Author reordering
  - **RECORD=225 (Bezier)**: 5 instances, LineStyleExt reordering
  - No missing parameters. No changed values.

### /Storage
- **Status**: DIFFERS
- **Size change**: 5,750 bytes -> 5,869 bytes (+119 bytes)
- **Diff category**: Key casing change (block[0]) + binary difference (block[1])
- **Details**:
  - **block[0]**: `WEIGHT=1` -> `Weight=1` (key casing)
  - **block[1]**: Binary data: 5,712 bytes -> 5,831 bytes (+119 bytes). Identical size change to files 04 and 05.

## Diff Categories Found

1. **Parameter key casing** (ALL CAPS -> MixedCase) -- all 10,343 differing blocks. Benign.
2. **Parameter reordering** -- RECORD=17 (69 blocks), RECORD=27 (384 blocks), RECORD=209 (4 blocks), RECORD=225 (5 blocks). Benign.
3. **Font parameter reordering** -- RECORD=31 (1 block). Benign.
4. **Binary differences** -- /Storage block[1] +119 bytes.

Categories NOT found:
- No missing parameters
- No changed values
- No new or missing streams

## Fidelity Assessment

**BENIGN**: All text-mode parameters preserved with only casing and ordering differences. The /Storage binary increase (+119 bytes) is consistent across files 04, 05, and 06, all of which share the same embedded LimeMicro logo BMP image, confirming a systematic serialization difference in the icon storage encoder.

## Impact on File Format Support

### Working well
- Identical assessment to file 05. All record types correctly handled.
- Cross-validation: Files 05 and 06 are the same schematic sheet (09_FPGA_misc) from two board variants (plug 1v2 vs socket 1v2). Both produce diff counts that match exactly (10,335 FileHeader blocks, 6 Additional blocks, 2 Storage blocks), confirming deterministic and consistent serialization behavior.

### What needs improvement
- Same cosmetic issues as files 04/05: parameter key casing, parameter ordering, font field ordering
- These are shared with all four files in this batch

### Specific bugs or missing features revealed
- **/Storage binary size**: +119 bytes across all three files using the same template. Root cause is in the binary image serialization path, not the text parameter path.
- No data-handling bugs.
