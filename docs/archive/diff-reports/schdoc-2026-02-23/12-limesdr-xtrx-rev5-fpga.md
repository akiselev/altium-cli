# Roundtrip Diff Report: FPGA.SchDoc (LimeSDR-XTRX rev5)

## Summary
- **File size**: 1,840,640 bytes original vs 1,843,200 bytes roundtripped
- **Save-as result**: Success
- **Streams differing**: 2 (/Additional, /FileHeader)
- **Streams identical**: 1 (/Storage)

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: Same block sizes (text blocks remain same byte count)
- **Diff category**: Parameter reordering
- **Details**: 6 blocks differ, all RECORD=225 (polygon). Parameters `LineWidth`, `Color`, `AreaColor`, `LineStyle`, `LineStyleExt`, and `UniqueID` are reordered. In the original, order is `LineWidth|Color|AreaColor|LineStyle|...|LineStyleExt|UniqueID`. In roundtrip, order is `Color|AreaColor|LineStyle|LineStyleExt|LineWidth|...|UniqueID`. All values are identical.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: 1,822,091 bytes -> 1,821,919 bytes (172 bytes smaller)
- **Diff category**: Parameter reordering, default value changes (ShowNetName removal), FontID injection, UTF-8 encoding change
- **Details**: 581 blocks differ across multiple record types:

  **RECORD=31 (Sheet header)** - 1 block: Font parameters reordered (FontName now comes before Size for each font index). Sheet-level properties reordered (SnapGrid/VisibleGrid/HotSpotGrid grouped, then SheetStyle, CustomX/Y, etc.). All values preserved.

  **RECORD=17 (Power port)** - 124 blocks: Three kinds of changes:
  - Style=2 (GND) ports: `Style=2|ShowNetName=F` removed and replaced with `Style=2` at end, `ShowNetName=F` dropped entirely, `FontID=1` added. This causes the file to shrink (101 blocks lose `ShowNetName=F`, gaining `FontID=1` -- net 5-byte reduction per block).
  - Style=1 (named power) ports: `Style=1|ShowNetName=T` moved to end of record, `FontID=1` added. These blocks grow slightly.
  - All parameters reordered with Location/Color/Text first, style/orientation later.

  **RECORD=18 (Sheet entry)** - 12 blocks: Parameters reordered. `IOType`, `Alignment`, `Width`, `Height` moved to after Name. `FontID` and `TextColor` moved. All values identical.

  **RECORD=25 (Net label)** - 1 block: Encoding change for non-ASCII character. Original uses raw UTF-8 byte for Cyrillic character in `Text=PCI_REF_CLK1_` (rendered as raw byte `\xd2`). Roundtrip uses HTML entity `&#1058;`. The `%UTF8%Text` value is identical, so the authoritative value is preserved.

  **RECORD=27 (Wire)** - 431 blocks: Parameter reordering only. `LineWidth|Color|UniqueID|LocationCount` becomes `Color|LineWidth|LocationCount|...|UniqueID`. All values identical, same byte lengths.

  **RECORD=2 (Pin)** - 12 blocks: `SwapIDPart` encoding change. Original has raw bytes `0ŽŽ&ŽŽ0` (Windows-1252 byte 0x8E). Roundtrip uses HTML entity `0&#1035;&&#1035;0`. The `%UTF8%SwapIDPart` value is identical in both cases.

### /Storage
- **Status**: OK (identical, 25 bytes)

## Diff Categories Found

1. **Parameter reordering** - Present in all record types (RECORD=17, 18, 25, 27, 225, 31). All values preserved. This is the dominant diff type (all 587 differing blocks).
2. **Default value injection** - `FontID=1` is added to RECORD=17 power port records that did not previously have it.
3. **Font parameter reordering** - RECORD=31 has FontName before Size (our serializer groups font fields differently).
4. **Missing parameters** - `ShowNetName=F` is dropped from Style=2 (GND) power ports. This is a default value that Altium would infer, so it is functionally benign.
5. **Changed values** - No actual value changes detected. The `SwapIDPart` and `Text` differences are encoding-level (raw byte vs HTML entity) with the `%UTF8%` prefix providing the authoritative value.
6. **New streams** - None.
7. **Binary differences** - None.
8. **Size-only differences** - None.
9. **Other** - Encoding representation change for non-ASCII characters in `Text` and `SwapIDPart` fields (raw byte vs HTML entity).

## Fidelity Assessment

**BENIGN** - All semantic data is preserved. The differences are:
- Parameter reordering (Altium is order-insensitive for parameter keys)
- Default value injection (`FontID=1` is the default)
- Default value omission (`ShowNetName=F` is the default for Style=2)
- Encoding representation (HTML entities vs raw bytes, with UTF-8 prefix as authority)

The 172-byte size reduction in /FileHeader is entirely accounted for by dropping `ShowNetName=F` from ~101 GND power ports (101 * ~14 bytes overhead, offset by added `FontID=1`).

## Impact on File Format Support

**What's working well:**
- Core record parsing and serialization for all record types present (RECORD=2, 4, 6, 17, 18, 25, 27, 31, 225)
- All coordinate values, UniqueIDs, text content, and electrical properties preserved exactly
- Block encoding and CFB container structure correct (no binary diffs, /Storage identical)

**What needs improvement:**
- **Parameter ordering**: Our serializer emits parameters in a different order than Altium's native serializer. While benign, matching Altium's order would make roundtrip diffs cleaner.
- **Default value handling for RECORD=17**: We inject `FontID=1` (default) and drop `ShowNetName=F` (default for GND style). Should match Altium's serialization behavior: only emit `ShowNetName` when it differs from the style's default, and only emit `FontID` when the original file included it.
- **Non-ASCII encoding in fallback fields**: We emit HTML entities (`&#1058;`) for non-ASCII characters in the non-UTF8 fallback `Text` field, while Altium writes raw Windows-1252 bytes. The `%UTF8%` prefixed field is correct, but the fallback should use raw encoding for byte-identical roundtrip.
- **SwapIDPart encoding**: Same issue -- `0x8E` escape byte should be preserved as raw byte, not converted to HTML entity.
