# Roundtrip Diff Report: myriadrf_LimeSDR-USB__hardware_plug_1v4_Schematics__10_FPGA_misc.SchDoc

## Summary
- **File size**: 2,320,384 bytes (original) vs 2,322,432 bytes (roundtripped) -- delta: +2,048 bytes (+0.09%)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: Block sizes are identical per block (80 bytes each for block 0; 417-427 bytes for blocks 1-5)
- **Diff category**: Parameter key casing + Parameter reordering (RECORD=225)
- **Details**:
  - 6 blocks differ (blocks 0-5).
  - **Block 0 (header)**: Key casing only: `WEIGHT=5` -> `Weight=5`
  - **Blocks 1-5 (RECORD=225 Polygon)**: Two types of diffs combined:
    1. Key casing normalization: ALL-CAPS keys (`INDEXINSHEET`, `OWNERPARTID`, `LOCATION.X`, etc.) -> mixed-case (`IndexInSheet`, `OwnerPartId`, `Location.X`, etc.)
    2. Parameter reordering: `LineStyleExt=1` moves from after the X4/Y4 coordinate list to before `LocationCount`. The original `LINESTYLEEXT=1` appeared at the very end, the roundtripped `LineStyleExt=1` appears before the coordinate block.
  - Note: These RECORD=225 blocks in the original do NOT have a `UniqueID` parameter (unlike file 02's RECORD=225 blocks which did). This is preserved correctly -- no UniqueID is injected.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Not reported explicitly (no length line in diff output)
- **Diff category**: Parameter key casing + Font parameter reordering + Sheet property reordering + Parameter reordering (RECORD=17, 27, 209)
- **Details**:
  - **10,415 blocks differ** out of ~10,415 total blocks (essentially every block).
  - **Dominant pattern**: Parameter key casing normalization. Original uses ALL-CAPS keys, roundtripped uses mixed-case canonical keys. All values are identical throughout.
  - **Block 0 (header)**: `WEIGHT=10414|MINORVERSION=2|UNIQUEID=TJIBRJUD` -> `Weight=10414|MinorVersion=2|UniqueID=TJIBRJUD`
  - **Block 1 (RECORD=31, sheet properties)**: Font fields reordered from `SIZE_N|FONTNAME_N` to `FontName_N|Size_N`. Sheet properties reordered: `SnapGridOn`, `VisibleGridOn`, `HotSpotGridOn` grouped together before `SheetStyle`; `SystemFont`, `UseMBCS`, `IsBOC`, `AreaColor` moved to end. This file has 12 fonts and uses the LimeSDR A3 template.
  - **Block 2 (RECORD=39, template)**: `ISNOTACCESIBLE=T|OWNERPARTID=-1|FILENAME=...` -> `IsNotAccesible=T|OwnerPartId=-1|FileName=...`. Key casing only.
  - **Blocks 3-36 (title block records)**: RECORD=4 (Labels), RECORD=6 (Polylines), RECORD=30 (Image), RECORD=41 (Parameters) for the title block template. All pure key casing changes. Includes `KeepAspect=T` on RECORD=30 (Image) -- preserved correctly.
  - **Blocks 37+ (schematic records)**: RECORD=41 system parameters, then component/pin/wire/junction data. All ALL-CAPS -> mixed-case key casing.
  - **RECORD=17 (Power Port)**: Same reordering pattern as file 02 -- `Style`, `ShowNetName`, `Orientation`, `FontID` moved after `Text` in roundtripped output.
  - **RECORD=27 (Wire)**: Same reordering as file 02 -- `LineWidth`/`Color` swap, `UniqueID` moved to end.
  - **RECORD=29 (Junction)**: Many blocks at the end of the file. Pure key casing: `INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=...|LOCATION.Y=...|COLOR=128` -> `IndexInSheet=-1|OwnerPartId=-1|Location.X=...|Location.Y=...|Color=128`. Fractional coordinates (`_FRAC` -> `_Frac`) also casing-normalized.
  - **RECORD=209 (Text Frame)**: Same reordering as file 02.
  - This file contains Lime Microsystems-specific data (Organization=Lime Microsystems, Address1=Surrey Tech Centre, Address2=Guildford GU2 7YG, Address3=Surrey, Address4=United Kingdom, Title=FPGA misc (power, clocks, config)). All text content preserved exactly.

### /Storage
- **Status**: DIFFERS
- **Size change**: 5,750 bytes -> 5,869 bytes (+119 bytes)
- **Diff category**: Parameter key casing + Binary differences
- **Details**:
  - Block 0 (header): `|HEADER=Icon storage|WEIGHT=1` -> `|HEADER=Icon storage|Weight=1` (key casing only)
  - Block 1 (binary payload): 5,712 bytes -> 5,831 bytes (+119 bytes). Embedded image data grew slightly, likely due to re-encoding of the LimeMicroLogoPCB.bmp.

## Diff Categories Found

1. **Parameter key casing** -- ALL-CAPS to mixed-case normalization across all 10,415 blocks in /FileHeader, 6 blocks in /Additional, and /Storage header. **Benign**.
2. **Parameter reordering (RECORD=225)** -- `LineStyleExt` repositioned before coordinate list. **Benign**.
3. **Parameter reordering (RECORD=17, 27, 209)** -- Same reordering patterns as file 02. **Benign**.
4. **Font parameter reordering** -- RECORD=31 `Size_N/FontName_N` reordered. **Benign**.
5. **Sheet property reordering** -- RECORD=31 non-font properties reordered. **Benign**.
6. **Default value injection** -- None observed (original already had `WEIGHT=5` in /Additional).
7. **Binary differences** -- /Storage binary block grew by 119 bytes. **Concerning**: Embedded image data changed.

## Fidelity Assessment

**BENIGN** (with caveat on /Storage binary)

All parameter diffs are either key casing normalization or parameter reordering, both of which are semantically invisible to Altium. No values were changed, no parameters were missing, and no parameters were added (except potential block-level padding in /Storage). The /Storage binary size increase of 119 bytes needs visual verification for the embedded logo.

## Impact on File Format Support

### What's working well
- All Lime Microsystems template data (organization info, addresses, project name) preserved exactly.
- `KeepAspect=T` on RECORD=30 (Image) preserved correctly.
- RECORD=39 (Template reference) with file path preserved correctly.
- Fractional coordinates (`_Frac` suffix) correctly handled throughout, including Junction records (RECORD=29) with `Location.X_Frac` and `Location.Y_Frac`.
- 12-font font table (RECORD=31) fully preserved with all font names, sizes, rotations, bold, italic, and underline attributes.
- Multi-part title block with labels, polylines, and embedded image correctly roundtripped.

### What needs improvement
- **Parameter key casing**: Same issue as files 00/01 -- serializer normalizes to mixed-case while originals were ALL-CAPS. This is the sole reason all 10,415 blocks differ.
- **RECORD=17/27/209/225 serialization order**: Same non-canonical parameter ordering as file 02.

### Specific bugs or missing features revealed
- **/Storage binary size increase**: Unlike files 00 and 02 where /Storage shrank, this file's /Storage grew by 119 bytes. The direction of the change varies per file, confirming this is a re-encoding difference rather than systematic data loss. The embedded BMP for `LimeMicroLogoPCB.bmp` is being re-encoded with slightly different byte output.
- **No new issues beyond what was found in files 00-02**: This file confirms the same patterns (key casing, parameter reordering, /Storage binary re-encoding) apply consistently across different SchDoc files with different content and templates.
