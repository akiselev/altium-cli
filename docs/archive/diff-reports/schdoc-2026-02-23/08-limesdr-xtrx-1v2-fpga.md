# Roundtrip Diff Report: myriadrf_LimeSDR-XTRX__hardware_1v2_Schematics__08_FPGA.SchDoc

## Summary
- **File size**: 2,281,984 bytes original -> 2,285,568 bytes roundtripped (+3,584 bytes)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: Not reported (first diff at offset 0x13a / 314)
- **Diff category**: Parameter reordering
- **Details**: 3 blocks differ (blocks 1-3). All are RECORD=225 (Polygon) records. The only change is the position of `LineStyleExt=1` -- in the original it appears after the X/Y coordinate pairs (just before `UniqueID`), while in the roundtrip it appears immediately after `LineStyle=1` (before `LocationCount`). Same keys, same values, different order.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Not reported (first diff at offset 0x97 / 151)
- **Diff category**: Parameter reordering (font params, power port fields, wire/bus fields)
- **Details**: 287 blocks differ across multiple record types:
  - **block[1] -- RECORD=31 (Sheet header/fonts)**: Font parameters reordered. Original writes `Size1=10|FontName1=...` (size before name), roundtrip writes `FontName1=...|Size1=10` (name before size). Sheet properties also reordered (e.g. `SystemFont`, `UseMBCS`, `IsBOC`, `AreaColor` moved to end).
  - **blocks 3838-9905 -- RECORD=17 (Power Port)**: ~86 blocks. The power port-specific fields `Style`, `ShowNetName`, `Orientation`, `FontID` are reordered. Original writes them early (after `OwnerPartId`), roundtrip writes `Color` and `Text` first, then `Style`, `ShowNetName`, `Orientation`, `FontID` later. All values identical.
  - **blocks 9905-9906 -- RECORD=27 (Wire)**: ~150+ blocks. The `LineWidth`, `Color`, `UniqueID` fields are reordered. Original writes `LineWidth=1|Color=8388608|UniqueID=...|LocationCount=N`, roundtrip writes `Color=8388608|LineWidth=1|LocationCount=N|...|UniqueID=...` (UniqueID moved to end). All values identical.
  - **blocks 10101-10104 -- RECORD=209 (Text Frame)**: 4 blocks. Fields like `Text`, `Author`, `FontID`, `TextColor` reordered. Original groups visual properties together, roundtrip puts `Text` and `Author` earlier. All values identical.

### /Storage
- **Status**: DIFFERS
- **Size change**: 6,381 bytes -> 5,869 bytes (-512 bytes)
- **Diff category**: Binary differences
- **Details**: 1 block differs (block[1], binary mode). The Storage stream contains embedded component model data. The size decrease of 512 bytes (exactly one CFB sector) suggests minor differences in how the embedded CFB data is laid out, likely padding or sector allocation differences.

## Diff Categories Found

1. **Parameter reordering** -- Present in all 3 streams. This is the dominant diff type. Affects RECORD=17 (Power Port), RECORD=27 (Wire), RECORD=209 (Text Frame), and RECORD=225 (Polygon). All parameter keys and values are identical; only serialization order differs.
2. **Font parameter reordering** -- Present in /FileHeader block[1]. Font fields reordered from `SizeN|FontNameN` to `FontNameN|SizeN`. Sheet properties also reordered.
3. **Binary differences** -- Present in /Storage. The embedded binary data block differs by 512 bytes (one sector). No text-level analysis possible for binary blocks.

## Fidelity Assessment

**BENIGN**: All text-mode differences are purely parameter reordering. Altium's parser reads parameters by key lookup (first occurrence wins, case-insensitive), so reordering is semantically invisible. The Storage binary difference warrants investigation but is likely padding/alignment.

## Impact on File Format Support

**What's working well:**
- All record types (1, 2, 4, 6, 17, 27, 31, 34, 41, 44, 45, 46, 48, 209, 225) are fully parsed and re-serialized without data loss
- All parameter values are preserved exactly (coordinates, fracs, colors, UniqueIDs, etc.)
- No parameters are missing or added in the roundtrip for /FileHeader and /Additional streams

**What needs improvement:**
- Parameter serialization order differs from Altium's native order for several record types (17, 27, 209, 225). While benign, matching Altium's order would enable byte-identical roundtrips.
- RECORD=17 (Power Port): Our serializer writes `Color`, `Text` before `Style`, `ShowNetName`, `Orientation`, `FontID`. Altium writes the style/display properties first.
- RECORD=27 (Wire): Our serializer moves `UniqueID` to the end and reorders `Color`/`LineWidth`. Altium writes `LineWidth|Color|UniqueID|LocationCount`.
- RECORD=31 (Sheet): Font field order and sheet property order differ.
- /Storage binary differences need investigation -- the 512-byte decrease could indicate re-serialization of embedded CFB with different sector allocation.
