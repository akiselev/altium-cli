# Roundtrip Diff Report: 08_FPGA.SchDoc (LimeSDR-XTRX 1v1)

## Summary
- **File size**: 2,281,984 bytes original -> 2,285,568 bytes roundtripped (+3,584 bytes)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: Block sizes identical per-block
- **Diff category**: Parameter reordering only (no key casing change)
- **Details**:
  - **block[0]**: Header block -- NOT differing (unlike files 04/05/06, this file already uses MixedCase keys)
  - **blocks[1-3]**: RECORD=225 (Bezier) records -- parameter reordering only:
    - `LineStyleExt=1` moved from after the Xn/Yn coordinate list to immediately after `LineStyle=1`
    - This file already uses MixedCase keys in the original, so no casing changes are needed
  - All parameter values identical. 3 blocks differ total.
  - This is notably different from files 04/05/06 which had ALL CAPS keys in the original. The LimeSDR-XTRX file was likely saved with a newer version of Altium Designer.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Block sizes identical per-block for all 286 differing blocks
- **Diff category**: Parameter reordering + font parameter reordering
- **Details**:
  - **block[0]**: Header -- NOT differing (already MixedCase)
  - **block[1]**: RECORD=31 (Sheet properties) -- font parameter reordering only:
    - Original: `Size1=10|FontName1=...` -> Roundtrip: `FontName1=...|Size1=10` (FontName emitted before Size)
    - 14 fonts defined (vs 12 in files 04/05/06), including Arial and Arial Narrow fonts
    - Sheet property reordering: `UseMBCS`, `IsBOC`, `SystemFont`, `AreaColor` moved to end in roundtrip
    - `HotSpotGridOn|HotSpotGridSize` group moved after `VisibleGrid` group
  - **blocks[3838-9943]**: Only 284 additional blocks differ (out of ~10,100 total blocks). This is far fewer than the ~10,400 differing blocks in files 04/05/06 because the original already uses MixedCase keys, so only records with actual parameter reordering show up as diffs.
  - **Differing record types** (only 5 types, all parameter reordering):
    - **RECORD=17 (PowerPort)**: 86 instances -- `Style=4|ShowNetName=T|Location.X=...|Orientation=3|Color=128|FontID=1|Text=GND` reordered to `Location.X=...|Color=128|Text=GND|Style=4|ShowNetName=T|Orientation=3|FontID=1`. Style/ShowNetName/Orientation move to after Text.
    - **RECORD=27 (Wire)**: 195 instances -- `LineWidth=1|Color=8388608|UniqueID=xxx|LocationCount=N` reordered to `Color=8388608|LineWidth=1|LocationCount=N|...|UniqueID=xxx`. LineWidth/Color swap; UniqueID moves to end.
    - **RECORD=209 (NoteText)**: 4 instances -- `AreaColor=...|TextColor=128|FontID=5|IsSolid=T|ShowBorder=T|WordWrap=T|ClipToRect=T|Text=...|TextMargin=5|Author=...` reordered to `AreaColor=...|Text=...|Author=...|FontID=5|TextColor=128|IsSolid=T|ShowBorder=T|WordWrap=T|ClipToRect=T|TextMargin=5`
    - **RECORD=225 (Bezier)**: Same LineStyleExt reordering as /Additional
    - **RECORD=31 (Sheet properties)**: Font/property reordering as described above
  - No missing parameters. No changed values.

### /Storage
- **Status**: DIFFERS
- **Size change**: 6,381 bytes -> 5,869 bytes (-512 bytes)
- **Diff category**: Binary difference
- **Details**:
  - **block[0]**: Header -- NOT differing (key already lowercase `Weight=1`)
  - **block[1]**: Binary data: 6,343 bytes -> 5,831 bytes (-512 bytes). Unlike files 04/05/06 where the binary grew by 119 bytes, this file's binary data SHRANK by 512 bytes. The original has a larger embedded image (the XTRX board uses a different or larger BMP than the LimeSDR-USB boards). The shrinkage suggests our serializer is re-encoding or recompressing the embedded image differently.

## Diff Categories Found

1. **Parameter reordering** -- affects RECORD=17 (86 blocks), RECORD=27 (195 blocks), RECORD=209 (4 blocks), RECORD=225 (3 blocks in /Additional). Same key=value pairs, different order. Benign.
2. **Font parameter reordering** -- RECORD=31 (1 block). FontName before Size; sheet properties regrouped. Benign.
3. **Binary differences** -- /Storage block[1] changed by -512 bytes.

Categories NOT found:
- No parameter key casing changes (original already uses MixedCase)
- No missing parameters
- No changed values
- No new or missing streams

## Fidelity Assessment

**BENIGN**: All text-mode parameter blocks contain identical key=value pairs with only ordering differences (no casing changes needed since the original already uses MixedCase). The /Storage binary size change (-512 bytes) is the only substantive difference and warrants investigation, as data shrinkage in embedded images could indicate loss of image fidelity.

## Impact on File Format Support

### Working well
- All record types correctly handled with only reordering diffs
- Far fewer diffs than files 04/05/06 (286 vs ~10,400 blocks) because the original already uses MixedCase keys -- proving our casing is compatible with newer Altium versions
- Only 5 record types have actual parameter ordering differences (RECORD=17, 27, 31, 209, 225), showing that most record serializers already emit parameters in the correct order
- Coordinates, fractional parts, Unicode escapes, and UniqueIDs all preserved correctly

### What needs improvement
- **RECORD=17 (PowerPort) parameter ordering**: Style/ShowNetName should be emitted before Location, not after Text
- **RECORD=27 (Wire) parameter ordering**: LineWidth should come before Color; UniqueID should come before LocationCount (not at end)
- **RECORD=209 (NoteText) parameter ordering**: Text/Author should come after the display properties (FontID, TextColor, etc.), not before them
- **RECORD=225 (Bezier/ClosedBezier)**: LineStyleExt should come after LocationCount and the coordinate list, not immediately after LineStyle
- **RECORD=31 (Sheet properties)**: FontName should come after Size within each font group; SystemFont/UseMBCS/IsBOC/AreaColor should not be moved to end

### Specific bugs or missing features revealed
- **/Storage binary size change**: -512 bytes (shrinkage). This is different from the +119 byte growth seen in files 04/05/06. The original image was 6,343 bytes and became 5,831 bytes. Notably, 5,831 is the same output size as files 04/05/06, suggesting our serializer is producing a fixed-size output regardless of input. This strongly suggests we are re-encoding/regenerating the embedded BMP image from parsed data rather than preserving the original bytes. This is a potential data loss issue for embedded images.
- **Consistent output size**: The roundtripped /Storage binary block is always 5,831 bytes across all 4 files (04, 05, 06, 07), despite originals being 5,712 bytes (04/05/06) and 6,343 bytes (07). This confirms the serializer is not preserving the original binary data.
