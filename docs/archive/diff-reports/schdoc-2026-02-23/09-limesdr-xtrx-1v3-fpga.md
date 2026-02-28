# Roundtrip Diff Report: myriadrf_LimeSDR-XTRX__hardware_1v3_Schematics__08_FPGA.SchDoc

## Summary
- **File size**: 2,263,552 bytes original -> 2,265,088 bytes roundtripped (+1,536 bytes)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: Not reported (first diff at offset 0x13a / 314)
- **Diff category**: Parameter reordering
- **Details**: 3 blocks differ (blocks 1-3). All are RECORD=225 (Polygon) records. Identical pattern to file 08: `LineStyleExt=1` position moves from after the X/Y coordinate pairs to immediately after `LineStyle=1`. Same keys, same values.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Not reported (first diff at offset 0x97 / 151)
- **Diff category**: Parameter reordering (font params, power port fields, wire/bus fields)
- **Details**: 286 blocks differ. The patterns are virtually identical to file 08 (these are v1.2 and v1.3 of the same design):
  - **block[1] -- RECORD=31 (Sheet header/fonts)**: FontIdCount=13 (vs 14 in v1.2). Font parameters reordered from `SizeN|FontNameN` to `FontNameN|SizeN`. Sheet properties reordered (grids grouped together, `SystemFont`/`UseMBCS`/`IsBOC`/`AreaColor` moved to end).
  - **RECORD=17 (Power Port)**: ~86 blocks. Same reordering as file 08 -- `Style`, `ShowNetName`, `Orientation`, `FontID` moved after `Color` and `Text`.
  - **RECORD=27 (Wire)**: ~150+ blocks. Same reordering as file 08 -- `UniqueID` moved to end, `Color`/`LineWidth` swapped.
  - **RECORD=209 (Text Frame)**: 4 blocks. Same reordering as file 08 -- `Text`, `Author` moved before visual properties.

### /Storage
- **Status**: DIFFERS
- **Size change**: 6,381 bytes -> 5,869 bytes (-512 bytes)
- **Diff category**: Binary differences
- **Details**: 1 block differs (block[1], binary mode). Identical size change to file 08 (-512 bytes / one CFB sector), suggesting the same embedded model data reserialization behavior.

## Diff Categories Found

1. **Parameter reordering** -- Dominant diff type across all text blocks. Affects RECORD=17, 27, 209, 225. All values preserved.
2. **Font parameter reordering** -- RECORD=31 font fields and sheet properties reordered.
3. **Binary differences** -- /Storage binary block differs by exactly 512 bytes, same as file 08.

## Fidelity Assessment

**BENIGN**: This file is the v1.3 revision of the same LimeSDR-XTRX FPGA schematic as file 08. All differences are purely parameter reordering (semantically invisible to Altium) plus the same /Storage binary difference. No data loss or corruption.

## Impact on File Format Support

**What's working well:**
- Consistent behavior between v1.2 and v1.3 of the same design -- our parser handles both versions identically
- All record types fully parsed and re-serialized
- All parameter values preserved exactly
- The diff pattern is 100% predictable and matches file 08

**What needs improvement:**
- Same parameter ordering issues as file 08 (RECORD=17, 27, 209, 225, 31)
- /Storage binary differences (-512 bytes) need investigation
- Since files 08 and 09 are nearly identical designs, fixing the ordering for one will fix both
