# Roundtrip Diff Report: ultrasonic-phased-array_hardware__fpga.SchDoc

## Summary
- **File size**: 14,265,344 bytes (original) vs 14,188,544 bytes (roundtripped) -- delta: -76,800 bytes (-0.54%)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: 75 bytes -> 84 bytes (+9 bytes)
- **Diff category**: Parameter key casing + Default value injection
- **Details**:
  - Original: `|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0`
  - Roundtripped: `|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0|Weight=0`
  - The serializer injects `Weight=0` (a default value) that was not present in the original.
  - 1 block differs.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Not reported explicitly (length line absent), but overall file size decreased. Based on block count, stream has ~79,543 blocks.
- **Diff category**: Parameter key casing + Parameter reordering (font fields) + Parameter reordering (RECORD=31 sheet properties)
- **Details**:
  - **79,543 blocks differ** out of ~79,543+ total blocks (essentially every block differs).
  - **Dominant pattern**: Parameter key casing normalization. Original uses ALL-CAPS keys (`OWNERINDEX`, `LOCATION.X`, `OWNERPARTID`, `FONTID`, etc.), roundtripped uses mixed-case canonical keys (`OwnerIndex`, `Location.X`, `OwnerPartId`, `FontID`, etc.). Same values throughout.
  - **Block 0 (header)**: `WEIGHT=79542|MINORVERSION=2|UNIQUEID=VYFDTJNL` -> `Weight=79542|MinorVersion=2|UniqueID=VYFDTJNL`
  - **Block 1 (RECORD=31, sheet properties)**: Font fields reordered. Original: `SIZE1=10|FONTNAME1=...` -> Roundtripped: `FontName1=...|Size1=10`. Also, non-font sheet properties reordered (e.g., `USEMBCS`, `ISBOC`, `SYSTEMFONT` moved to end). Same keys and values, different order.
  - **Blocks 2+ (records)**: Pure key casing change for all record types encountered (RECORD=4, 6, 30, 39, 41, 1, 2, 14, 34, 44, 45, 46, 48, 29, etc.). No value changes detected.
  - Record types seen: 1 (Component), 2 (Pin), 4 (Label), 6 (Polyline), 14 (Rectangle), 17 (Power Port), 25 (Net Label), 27 (Wire), 29 (Junction), 30 (Image), 31 (Sheet Properties), 34 (Designator), 39 (Template), 41 (Parameter), 44 (Implementation Map), 45 (Implementation), 46 (Implementation Pin Assoc), 48 (Implementation Child).

### /Storage
- **Status**: DIFFERS
- **Size change**: 10,319 bytes -> 9,933 bytes (-386 bytes)
- **Diff category**: Parameter key casing + Binary differences
- **Details**:
  - Block 0 (header): `|HEADER=Icon storage|WEIGHT=1` -> `|HEADER=Icon storage|Weight=1` (key casing only)
  - Block 1 (binary payload): 10,281 bytes vs 9,895 bytes. This is embedded image/icon data. The size difference of -386 bytes is the primary contributor to the overall file shrinkage. Likely due to re-encoding of embedded bitmap data.

## Diff Categories Found

1. **Parameter key casing** -- ALL-CAPS keys normalized to mixed-case canonical form. This is the dominant diff, affecting all 79,543+ blocks in /FileHeader and the headers of /Additional and /Storage. **Benign**: Altium's parameter parser is case-insensitive.
2. **Default value injection** -- `Weight=0` added to /Additional header block that was absent in original. **Benign**: This is a default value that Altium would assume anyway.
3. **Font parameter reordering** -- In RECORD=31 (sheet properties), font fields are reordered from `SIZE_N|FONTNAME_N` to `FontName_N|Size_N`, and non-font properties are also reordered. **Benign**: Altium does not depend on parameter order.
4. **Binary differences** -- /Storage binary block is 386 bytes smaller. **Concerning**: Embedded image data size changed, possibly due to re-encoding (e.g., different BMP compression or padding). Needs investigation to confirm no visual data loss.

## Fidelity Assessment

**BENIGN** (with caveat on /Storage binary)

The vast majority of diffs (79,543+ blocks) are purely parameter key casing normalization, which is semantically invisible to Altium. The font/sheet property reordering is also benign. The only potential concern is the /Storage binary block size change (-386 bytes), which affects embedded icon/image data and may warrant visual verification.

## Impact on File Format Support

### What's working well
- All schematic record types are being parsed and re-serialized correctly (no missing parameters, no changed values).
- Parameter values are preserved exactly for all record types tested (1, 2, 4, 6, 14, 17, 25, 27, 29, 30, 31, 34, 39, 41, 44, 45, 46, 48).
- UTF-8 encoded parameters (`%UTF8%SwapIDPart`) are preserved correctly.
- Special characters in pin names (backslash overbar notation like `S\R\C\L\R\`) are preserved.
- Fractional coordinate values are not present in this file (integer-only coordinates), so no _Frac testing here.

### What needs improvement
- **Parameter key casing**: The serializer outputs canonical mixed-case keys while the original file used ALL-CAPS. This is semantically benign but produces large diff output. Consider preserving original casing for byte-identical roundtrips, or accept this as an intentional normalization (Altium itself normalizes to mixed-case on save).
- **Default value injection**: `Weight=0` is injected into /Additional even when not present in the original. Consider suppressing default-valued parameters when they match the implicit default.
- **Font parameter ordering**: Font fields within RECORD=31 are serialized in a different order than the original. The serializer puts `FontName` before `Size` (matching Altium's canonical order), while the original had `Size` before `FontName`.

### Specific bugs or missing features revealed
- **/Storage binary size change**: The embedded icon data in /Storage changed size (10,281 -> 9,895 bytes). This may indicate a difference in how embedded BMP data is being re-encoded. Should be investigated to ensure no visual data loss.
- **Sheet property reordering**: Non-font properties in RECORD=31 (like `UseMBCS`, `IsBOC`, `SystemFont`, `AreaColor`) are moved to the end of the parameter list. This is benign but prevents byte-identical roundtrips.
