# Roundtrip Diff Report: myriadrf_LimeSDR-XTRX__hardware_1v2_Schematics__09_Misc.SchDoc

## Summary
- **File size**: 1,673,216 bytes original -> 1,679,360 bytes roundtripped (+6,144 bytes)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: Same-length blocks only (no length line reported)
- **Blocks differing**: 4 (blocks 1-4)
- **Diff category**: Parameter reordering
- **Details**: All 4 differing blocks are RECORD=225 (Polygon) records. Identical to file 16 (LimeSDR-XTRX 1v3): `LineStyleExt=1` moves from after the coordinate list to immediately after `LineStyle=1`. All values preserved.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: 1,649,457 bytes -> 1,649,443 bytes (-14 bytes)
- **Blocks differing**: 352
- **Diff category**: Parameter reordering, font parameter reordering, missing parameters (default value omission)
- **Details**: The 352 differing blocks break down as follows:

  **Block 1 (RECORD=31 -- Sheet Header)**: Font parameter reordering (same pattern as files 16 and 17). 11 fonts with FontName moved before Size. Sheet-level settings reordered. All values identical.

  **~56 blocks of RECORD=17 (Power Object)**: Parameter reordering. Same pattern as file 16: `Style`, `ShowNetName`, coordinates, `Orientation`, `Color`, `FontID`, `Text` reorder.

  **~240+ blocks of RECORD=27 (Wire)**: Parameter reordering. `LineWidth`/`Color` swap, `UniqueID` moves to end.

  **~10 blocks of RECORD=209 (Text Frame)**: Parameter reordering. Same pattern as file 16.

  **1 block of RECORD=22 (No ERC Marker)**: **Missing parameter**: `SuppressAll=F` is present in the original but absent in the roundtrip. Original: `...IsActive=T|SuppressAll=F|ConnectionPairsToSuppress=PNO_PNR|UniqueID=...`. Roundtrip: `...IsActive=T|ConnectionPairsToSuppress=PNO_PNR|UniqueID=...`. Block size changed: 235 bytes -> 221 bytes (-14 bytes, accounting for the full FileHeader size shrinkage). This is a default value being omitted -- `SuppressAll=F` is the default for No ERC markers.

  **~35+ blocks of RECORD=17 (Power Object) with additional reordering**: Standard reordering.

### /Storage
- **Status**: DIFFERS
- **Size change**: 6,381 bytes -> 5,869 bytes (-512 bytes)
- **Blocks differing**: 1 (block 1)
- **Diff category**: Binary differences
- **Details**: Single binary block: 6,343 -> 5,831 bytes. Embedded component graphics re-encoded during roundtrip.

## Diff Categories Found

1. **Parameter reordering** -- Affects RECORD=17 (Power Object), RECORD=27 (Wire), RECORD=209 (Text Frame), RECORD=225 (Polygon). All values preserved. Benign.
2. **Font parameter reordering** -- RECORD=31 (Sheet Header). Benign.
3. **Missing parameters** -- `SuppressAll=F` dropped from 1 RECORD=22 (No ERC Marker) block. This is a default value (`F` = false), so omitting it should be semantically equivalent. However, the original file explicitly stored it. **CONCERNING** -- while likely benign, our serializer should ideally preserve explicitly-set default values to achieve byte-identical roundtrips.
4. **Binary differences** -- /Storage stream binary block shrank by 512 bytes.

## Fidelity Assessment

**CONCERNING**: Most differences are benign parameter reordering, but the `SuppressAll=F` omission in RECORD=22 is a missing parameter. Although `F` is the default value and Altium would interpret the absence identically, this represents a loss of explicit data from the original file.

## Impact on File Format Support

**What's working well:**
- This file is very similar to file 16 (LimeSDR-XTRX 1v3) and shows the same consistent patterns, indicating stable parser/serializer behavior
- All RECORD=225 (Polygon) records with complex coordinate lists preserve all values
- All RECORD=209 (Text Frame) records preserve all text content, Author, and display settings
- Power Object records preserve all style, orientation, and color values

**What needs improvement:**
- Parameter serialization order for RECORD=17, RECORD=27, RECORD=209, RECORD=225 does not match Altium's native order
- Font field ordering in RECORD=31 differs from original
- `SuppressAll=F` is dropped from RECORD=22 when it is the default value. The serializer should explicitly write boolean fields that were present in the original, even if they equal the default.
- /Storage binary block size changes by -512 bytes

**Specific bugs revealed:**
- **Default value omission bug**: RECORD=22 (No ERC Marker) drops `SuppressAll=F` during serialization because it matches the default value. The serializer should preserve this field when it was explicitly present in the source data.
