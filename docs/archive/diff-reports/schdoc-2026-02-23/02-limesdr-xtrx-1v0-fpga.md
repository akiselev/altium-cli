# Roundtrip Diff Report: myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__08_FPGA.SchDoc

## Summary
- **File size**: 2,376,704 bytes (original) vs 2,379,776 bytes (roundtripped) -- delta: +3,072 bytes (+0.13%)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: Not reported (no explicit length line), but block sizes are identical for differing blocks
- **Diff category**: Parameter reordering (RECORD=225)
- **Details**:
  - 3 blocks differ (blocks 1, 2, 3), all RECORD=225 (Polygon) records.
  - The diff is purely **parameter reordering**: `LineStyleExt=1` moves from after the `X4/Y4` coordinate list to before `LocationCount`. `UniqueID` moves from end to after coordinate list.
  - Original order: `...LineStyle=1|LocationCount=4|X1=...|...|X4_Frac=...|LineStyleExt=1|UniqueID=...`
  - Roundtripped order: `...LineStyle=1|LineStyleExt=1|LocationCount=4|X1=...|...|X4_Frac=...|UniqueID=...`
  - All values are identical. Key casing is already mixed-case in the original (this file was likely saved by a newer Altium version).
  - Block 0 (header) is NOT listed as differing, meaning the /Additional header block is byte-identical.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Not reported explicitly, but block sizes within each block are identical (no per-block size changes)
- **Diff category**: Font parameter reordering + Parameter reordering (RECORD=17, 27, 209)
- **Details**:
  - **294 blocks differ** out of ~10,507+ total blocks (~2.8% of blocks differ).
  - This file already uses mixed-case parameter keys in the original (no ALL-CAPS casing issue).
  - **Block 1 (RECORD=31, sheet properties)**: Font fields reordered from `Size_N|FontName_N` to `FontName_N|Size_N`. Sheet-level properties reordered: `UseMBCS`, `IsBOC`, `SystemFont`, `AreaColor` moved to end. Same keys and values.
  - **RECORD=17 (Power Port)**: ~91 blocks. Parameters reordered: Original has `Style=N|ShowNetName=T` immediately after `OwnerPartId`, then location fields, then `Orientation`, `Color`, `FontID`, `Text`. Roundtripped moves `Style`, `ShowNetName`, `Orientation`, `FontID` after `Text`. Example:
    - Original: `|RECORD=17|...|Style=4|ShowNetName=T|Location.X=...|Orientation=3|Color=128|FontID=1|Text=GND|UniqueID=...`
    - Roundtripped: `|RECORD=17|...|Location.X=...|Color=128|Text=GND|Style=4|ShowNetName=T|Orientation=3|FontID=1|UniqueID=...`
  - **RECORD=27 (Wire)**: ~198 blocks. Parameters reordered: `LineWidth` and `Color` swap positions, and `UniqueID` moves from middle to end. Example:
    - Original: `|RECORD=27|...|LineWidth=1|Color=8388608|UniqueID=...|LocationCount=2|X1=...|Y2=...`
    - Roundtripped: `|RECORD=27|...|Color=8388608|LineWidth=1|LocationCount=2|X1=...|Y2=...|UniqueID=...`
  - **RECORD=209 (Text Frame)**: 4 blocks. Parameters reordered: `Text`, `Author` moved before `FontID`, `TextColor`, etc. Example:
    - Original: `|RECORD=209|...|AreaColor=9895935|TextColor=128|FontID=6|IsSolid=T|ShowBorder=T|WordWrap=T|ClipToRect=T|Text=FPGA|TextMargin=5|Author=DR`
    - Roundtripped: `|RECORD=209|...|AreaColor=9895935|Text=FPGA|Author=DR|FontID=6|TextColor=128|IsSolid=T|ShowBorder=T|WordWrap=T|ClipToRect=T|TextMargin=5`

### /Storage
- **Status**: DIFFERS
- **Size change**: 6,381 bytes -> 5,869 bytes (-512 bytes)
- **Diff category**: Binary differences
- **Details**:
  - Block 1 (binary payload): 6,343 bytes vs 5,831 bytes (-512 bytes).
  - This is embedded image/icon data that changed size, likely due to re-encoding of the LimeMicro logo BMP.
  - The /Storage header block (block 0) was not listed as differing, suggesting it is byte-identical.

## Diff Categories Found

1. **Parameter reordering** -- Parameters within records (RECORD=17, 27, 209, 225) are reordered. Same keys and values, different serialization order. **Benign**: Altium's parser is order-insensitive.
2. **Font parameter reordering** -- RECORD=31 font fields reordered (Size before FontName -> FontName before Size). **Benign**.
3. **Sheet property reordering** -- RECORD=31 non-font properties reordered (SystemFont, UseMBCS, IsBOC, AreaColor moved to end). **Benign**.
4. **Binary differences** -- /Storage binary block is 512 bytes smaller. **Concerning**: Embedded image data size changed.

## Fidelity Assessment

**BENIGN** (with caveat on /Storage binary)

This is the best result in the batch. Only 294 out of ~10,507 blocks differ (2.8%), and all diffs are parameter reordering within blocks. The original file already used mixed-case keys, so there is no casing normalization noise. The only concern is the /Storage binary size change (-512 bytes) for embedded image data.

## Impact on File Format Support

### What's working well
- This file was already saved with mixed-case parameter keys, and the roundtrip preserves them -- confirming the serializer matches Altium's canonical key casing.
- ~97.2% of blocks are byte-identical, indicating excellent fidelity for most record types.
- Fractional coordinates (`_Frac` parameters) are preserved correctly throughout.
- RECORD=225 (Polygon) parameters are all preserved, just reordered.
- RECORD=17 (Power Port) values including Style, ShowNetName, Orientation, FontID all preserved correctly.
- RECORD=209 (Text Frame) with Author, TextMargin, ClipToRect, WordWrap all preserved correctly.

### What needs improvement
- **RECORD=17 (Power Port) serialization order**: The serializer outputs `Style`, `ShowNetName`, `Orientation`, `FontID` in a different position than Altium's canonical order. This affects 91 blocks in this file.
- **RECORD=27 (Wire) serialization order**: `LineWidth`/`Color` order differs from original, and `UniqueID` is moved to end. Affects 198 blocks.
- **RECORD=209 (Text Frame) serialization order**: `Text` and `Author` are serialized before visual properties (`FontID`, `TextColor`) instead of after.
- **RECORD=225 (Polygon) serialization order**: `LineStyleExt` is serialized before `LocationCount` instead of after the coordinate list.

### Specific bugs or missing features revealed
- **Parameter serialization ordering for RECORD=17, 27, 209, 225**: These four record types have non-canonical parameter ordering in the serializer. While benign for Altium parsing, fixing the order would reduce diff noise and move toward byte-identical roundtrips.
- **/Storage binary shrinkage**: -512 bytes in embedded icon data. Same concern as file 00 -- needs visual verification.
- **Notably**: No default value injection (`Weight=0`) in /Additional for this file, suggesting the original already had the Weight parameter (the /Additional header block was byte-identical).
