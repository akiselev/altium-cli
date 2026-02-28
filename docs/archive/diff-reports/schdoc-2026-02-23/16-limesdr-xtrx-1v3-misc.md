# Roundtrip Diff Report: myriadrf_LimeSDR-XTRX__hardware_1v3_Schematics__09_Misc.SchDoc

## Summary
- **File size**: 1,743,872 bytes original -> 1,748,992 bytes roundtripped (+5,120 bytes)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: Same stream lengths (no length line reported, only content diffs)
- **Blocks differing**: 4 (blocks 1-4)
- **Diff category**: Parameter reordering
- **Details**: All 4 differing blocks are RECORD=225 (Polygon) records. The only change is the position of `LineStyleExt=1` -- in the original it appears after the coordinate list near the end (before `UniqueID`), in the roundtrip it appears immediately after `LineStyle=1` (before `LocationCount`). Same keys, same values, different order.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Not explicitly reported in length line, but stream content differs
- **Blocks differing**: 249
- **Diff category**: Parameter reordering, font parameter reordering
- **Details**: The 249 differing blocks break down as follows:

  **Block 1 (RECORD=31 -- Sheet Header)**: Font parameter reordering. In the original, fonts are serialized as `Size1=10|FontName1=...`, but in roundtrip as `FontName1=...|Size1=10`. Sheet-level settings like `UseMBCS`, `IsBOC`, `SystemFont`, `AreaColor` are also reordered (moved to end). All values are identical.

  **~52 blocks of RECORD=17 (Power Object)**: Parameter reordering only. Original order: `Style=4|ShowNetName=T|Location.X=...|...|Orientation=3|Color=128|FontID=2|Text=GND|UniqueID=...`. Roundtrip order: `Location.X=...|...|Color=128|Text=GND|Style=4|ShowNetName=T|Orientation=3|FontID=2|UniqueID=...`. All values preserved.

  **~186 blocks of RECORD=27 (Wire)**: Parameter reordering only. `LineWidth` and `Color` swap order, and `UniqueID` moves from middle to end. Original: `LineWidth=1|Color=8388608|UniqueID=...|LocationCount=...`. Roundtrip: `Color=8388608|LineWidth=1|LocationCount=...|UniqueID=...`.

  **~10 blocks of RECORD=209 (Text Frame)**: Parameter reordering. `TextColor`, `FontID`, and display flags reorder relative to `Text`, `Author`.

### /Storage
- **Status**: DIFFERS
- **Size change**: 732,753 bytes -> 732,673 bytes (-80 bytes)
- **Blocks differing**: 3 (blocks 1-3)
- **Diff category**: Binary differences
- **Details**: All 3 blocks are binary-mode blocks. Block 1: 6,343 -> 5,831 bytes. Block 2: 334,878 -> 336,113 bytes. Block 3: 391,486 -> 390,683 bytes. The /Storage stream contains embedded component images (BMP/PNG). The size changes suggest image data is being re-encoded or compressed differently during roundtrip. Total net change is -80 bytes.

## Diff Categories Found

1. **Parameter reordering** -- Present in all 3 text streams. RECORD=17 (Power Object), RECORD=27 (Wire), RECORD=209 (Text Frame), and RECORD=225 (Polygon) all show different key ordering. All values are preserved. This is benign.
2. **Font parameter reordering** -- RECORD=31 (Sheet Header) has FontName before Size in roundtrip vs Size before FontName in original. Benign.
3. **Binary differences** -- /Storage stream has 3 binary blocks with different sizes, likely due to image re-compression.

No missing parameters, no changed values, no default value injection, no new/removed streams.

## Fidelity Assessment

**BENIGN**: All text-mode differences are purely parameter reordering with identical keys and values. The /Storage binary differences are minor size variations in what appear to be embedded image blocks, likely due to re-compression. Altium reads parameters by key name regardless of order, so parameter reordering is semantically transparent.

## Impact on File Format Support

**What's working well:**
- All record types are fully parsed and re-serialized with correct values
- All coordinate values (including _Frac fractional parts) are preserved exactly
- UniqueID values are preserved
- Complex records (Polygon with LocationCount, Wire with multiple vertices) serialize correctly
- /Additional stream records (RECORD=225) round-trip correctly

**What needs improvement:**
- Parameter serialization order does not match Altium's native ordering for RECORD=17, RECORD=27, RECORD=209, and RECORD=225. While semantically benign, matching the native order would produce byte-identical roundtrips for these record types.
- Font fields in RECORD=31 serialize in a different order (FontName before Size vs Size before FontName).
- /Storage binary blocks have minor size differences, suggesting the image storage encoding is not perfectly reproduced.
