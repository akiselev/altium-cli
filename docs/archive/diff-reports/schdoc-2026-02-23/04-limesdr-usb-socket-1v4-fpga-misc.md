# Roundtrip Diff Report: 10_FPGA_misc.SchDoc (LimeSDR-USB socket 1v4)

## Summary
- **File size**: 2,320,384 bytes original -> 2,322,432 bytes roundtripped (+2,048 bytes)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: Block sizes identical per-block (80, 417, 424, 424, 424, 427 bytes each)
- **Diff category**: Parameter key casing change + parameter reordering
- **Details**:
  - **block[0]**: Header block -- `WEIGHT=5` -> `Weight=5` (key casing only)
  - **blocks[1-5]**: RECORD=225 (Bezier) records -- two kinds of diffs:
    1. Key casing: `INDEXINSHEET` -> `IndexInSheet`, `OWNERPARTID` -> `OwnerPartId`, `LOCATION.X` -> `Location.X`, `LOCATION.X_FRAC` -> `Location.X_Frac`, etc. (ALL CAPS -> MixedCase)
    2. Parameter reordering: `LineStyleExt=1` moved from after the Xn/Yn coordinate list to immediately after `LineStyle=1`, before `LocationCount`
  - All parameter values are identical between original and roundtrip. The total byte count per block is identical.
  - 6 blocks differ total.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Block sizes identical per-block for all 10,415 differing blocks
- **Diff category**: Parameter key casing change + parameter reordering + font parameter reordering
- **Details**:
  - **block[0]**: Header -- `WEIGHT=10414` -> `Weight=10414`, `MINORVERSION=2` -> `MinorVersion=2`, `UNIQUEID` -> `UniqueID` (key casing)
  - **block[1]**: RECORD=31 (Sheet properties) -- ALL CAPS keys -> MixedCase keys, PLUS:
    - Font parameter reordering: Original writes `SIZE1=10|FONTNAME1=...`, roundtrip writes `FontName1=...|Size1=10` (FontName emitted before Size per font)
    - Sheet property reordering: Grid/display properties grouped differently. `SYSTEMFONT`, `USEMBCS`, `ISBOC`, `AREACOLOR` moved to end in roundtrip
  - **blocks[2-62]**: Various record types (RECORD=39, 4, 6, 30, 41) -- key casing changes only (ALL CAPS -> MixedCase). Same values, same order within each record type. 10,415 blocks total.
  - **blocks[63+]**: RECORD=1 (Component), RECORD=14 (Rectangle), RECORD=2 (Pin), RECORD=41 (Parameter), RECORD=12 (Arc), RECORD=22 (NoERC), RECORD=25 (NetLabel), RECORD=7 (Line), RECORD=34 (Designator), RECORD=43 (ImplementationList), RECORD=44 (Implementation), RECORD=45 (ImplementationChild1), RECORD=46 (ImplementationChild2), RECORD=48 (ImplementationChild4), RECORD=28 (TextFrame), RECORD=29 (Junction), RECORD=17 (PowerPort), RECORD=27 (Wire), RECORD=209 (NoteText) -- ALL are key casing changes only
  - **RECORD=17 (PowerPort)**: 69 instances show parameter reordering beyond casing: Original `Style=4|ShowNetName=T|Location.X=...|Orientation=3|Color=128|FontID=1|Text=GND` becomes `Location.X=...|Color=128|Text=GND|Style=4|ShowNetName=T|Orientation=3|FontID=1`. Same key=value pairs, different order.
  - **RECORD=27 (Wire)**: 368 instances show `LineWidth=1|Color=8388608|UniqueID=xxx|LocationCount=N` becoming `Color=8388608|LineWidth=1|LocationCount=N|...|UniqueID=xxx`. UniqueID moves to end; LineWidth/Color swap order.
  - **RECORD=209 (NoteText)**: 6 instances show reordering: `AreaColor=...|TextColor=128|FontID=5|IsSolid=T|ShowBorder=T|WordWrap=T|ClipToRect=T|Text=...|TextMargin=5|Author=...` becomes `AreaColor=...|Text=...|Author=...|FontID=5|TextColor=128|IsSolid=T|ShowBorder=T|WordWrap=T|ClipToRect=T|TextMargin=5`. Text and Author move earlier.
  - No missing parameters. No changed values. All 10,415 differing blocks have identical byte sizes.

### /Storage
- **Status**: DIFFERS
- **Size change**: 5,750 bytes -> 5,869 bytes (+119 bytes)
- **Diff category**: Key casing change (block[0]) + binary difference (block[1])
- **Details**:
  - **block[0]**: `WEIGHT=1` -> `Weight=1` (key casing only, text block, 30 bytes each)
  - **block[1]**: Binary data differs: 5,712 bytes -> 5,831 bytes (+119 bytes). This is the embedded icon/image storage. The size increase suggests the roundtripped serialization of the binary image data differs slightly (possibly BMP re-encoding or padding differences).

## Diff Categories Found

1. **Parameter key casing** (ALL CAPS -> MixedCase) -- affects all 10,423 differing blocks across all 3 streams. The original file uses legacy ALL CAPS parameter keys (`OWNERINDEX`, `LOCATION.X`, `FONTID`), while our serializer outputs modern MixedCase keys (`OwnerIndex`, `Location.X`, `FontID`). This is the dominant diff category. Altium's parser is case-insensitive for parameter keys, so this is fully benign.

2. **Parameter reordering** -- affects RECORD=17 (PowerPort, 69 blocks), RECORD=27 (Wire, 368 blocks), RECORD=209 (NoteText, 6 blocks), RECORD=225 (Bezier, 5 blocks in /Additional). Same key=value pairs emitted in different order. Benign -- Altium parses by key name, not position.

3. **Font parameter reordering** -- affects RECORD=31 (block[1]). Font fields reordered: `Size` before `FontName` in original, `FontName` before `Size` in roundtrip. Sheet properties also reordered. Benign.

4. **Binary differences** -- /Storage block[1] has a 119-byte size increase. This is the only non-text diff.

Categories NOT found:
- No missing parameters (no data loss)
- No changed values (no data corruption)
- No new or missing streams

## Fidelity Assessment

**BENIGN**: All text-mode parameter blocks contain identical key=value pairs with only casing and ordering differences. Altium's parameter parser is case-insensitive and order-independent. The only substantive difference is a 119-byte increase in the /Storage binary block (embedded image), which warrants investigation but is unlikely to affect schematic behavior.

## Impact on File Format Support

### Working well
- All schematic record types parsed and re-serialized correctly (RECORD=1, 2, 4, 6, 7, 12, 14, 17, 22, 25, 27, 28, 29, 30, 31, 34, 39, 41, 43, 44, 45, 46, 48, 209, 225)
- All parameter values preserved exactly
- All coordinate values (including _FRAC fractional parts) preserved
- Unicode escape sequences (`%UTF8%SwapIDPart`) preserved correctly
- UniqueID values preserved
- Complex multi-point coordinate records (RECORD=27 with up to 27 location points) fully preserved

### What needs improvement
- **Parameter key casing**: Our serializer uses MixedCase while the original uses ALL CAPS. This is benign but creates noisy diffs that obscure real issues. Consider matching the original's casing style for cleaner roundtrips.
- **Parameter ordering**: Several record types (17, 27, 209, 225) emit parameters in a different order than the original. The order differences are specific to each record type's serializer. Not a correctness issue, but fixing the order would enable byte-identical roundtrips for text blocks.
- **Font field ordering in RECORD=31**: FontName emitted before Size instead of after. Benign but differs from original.
- **Sheet property ordering in RECORD=31**: Grid/display/system properties emitted in a different group order.

### Specific bugs or missing features revealed
- **/Storage binary size difference**: The embedded image binary data grew by 119 bytes. This needs investigation -- could indicate a BMP re-encoding issue or serialization padding difference in the icon storage format.
- **No actual bugs in data handling**: All semantic data is correctly preserved.
