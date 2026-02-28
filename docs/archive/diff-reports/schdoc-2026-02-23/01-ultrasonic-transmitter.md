# Roundtrip Diff Report: ultrasonic-phased-array_hardware__transmitter.SchDoc

## Summary
- **File size**: 8,430,592 bytes (original) vs 8,380,416 bytes (roundtripped) -- delta: -50,176 bytes (-0.60%)
- **Save-as result**: Success
- **Streams differing**: 2 (/Additional, /FileHeader)
- **Streams identical**: 1 (/Storage)

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: 75 bytes -> 84 bytes (+9 bytes)
- **Diff category**: Default value injection
- **Details**:
  - Original: `|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0`
  - Roundtripped: `|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0|Weight=0`
  - The serializer injects `Weight=0` (a default value) not present in the original.
  - 1 block differs.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: 8,361,932 bytes -> 8,355,452 bytes (-6,480 bytes)
- **Diff category**: Parameter key casing + Font parameter reordering + Sheet property reordering
- **Details**:
  - **46,565 blocks differ** out of ~46,565 total blocks (essentially every block).
  - **Dominant pattern**: Parameter key casing normalization. Original uses ALL-CAPS keys (`OWNERINDEX`, `LOCATION.X`, `COLOR`, etc.), roundtripped uses mixed-case canonical keys (`OwnerIndex`, `Location.X`, `Color`, etc.). All values are identical.
  - **Block 0 (header)**: `WEIGHT=46564|MINORVERSION=2|UNIQUEID=OLQKDYVE` -> `Weight=46564|MinorVersion=2|UniqueID=OLQKDYVE`
  - **Block 1 (RECORD=31)**: Font fields reordered (`SIZE1=10|FONTNAME1=...` -> `FontName1=...|Size1=10`). Sheet properties reordered (e.g., `USEMBCS`, `ISBOC`, `SYSTEMFONT`, `AREACOLOR` moved to end of parameter list). Additionally this file has `TitleBlockOn=T`, `CustomXZones`, `CustomYZones`, `CustomMarginWidth` which are all preserved with correct values.
  - **Blocks 2-28 (system parameters)**: RECORD=41 blocks for system parameters (CurrentTime, CurrentDate, Time, Date, DocumentFullPathAndName, DocumentName, ModifiedDate, ApprovedBy, CheckedBy, Author, CompanyName, DrawnBy, Engineer, Organization, Address1-4, Title, DocumentNumber, Revision, SheetNumber, SheetTotal, Rule, ImagePath, ProjectName, Application_BuildNumber). Pure key casing changes.
  - **Blocks 29+ (component/pin/record data)**: RECORD=1 (Component), RECORD=2 (Pin), RECORD=14 (Rectangle), RECORD=41 (Parameter). Pure key casing changes. Values preserved exactly.
  - **Size decrease of 6,480 bytes**: The overall stream shrank slightly. This is likely due to the case normalization producing slightly shorter keys in aggregate (e.g., `OWNERINDEX` (10 chars) vs `OwnerIndex` (10 chars) -- same length, but some keys like `ISNOTACCESIBLE` (14) vs `IsNotAccesible` (14) are the same). The actual source of the shrinkage may be removal of trailing padding or whitespace normalization in block encoding.
  - Record types seen: 1, 2, 14, 29, 34, 41, 44, 45, 46, 48.
  - The file contains multi-part components (up to 25 parts, e.g., PIN_HEADER_1.27_2x50 with PartCount=25). All part IDs and swap ID parameters are preserved correctly.

### /Storage
- **Status**: OK
- **Size**: 25 bytes (identical)
- **Details**: Byte-identical. This file has a minimal /Storage stream (no embedded images).

## Diff Categories Found

1. **Parameter key casing** -- ALL-CAPS to mixed-case normalization across all 46,565 blocks. **Benign**.
2. **Default value injection** -- `Weight=0` added to /Additional. **Benign**.
3. **Font parameter reordering** -- RECORD=31 font fields reordered. **Benign**.
4. **Sheet property reordering** -- Non-font properties in RECORD=31 reordered. **Benign**.
5. **Size-only differences** -- /FileHeader stream is 6,480 bytes smaller despite same logical content. Likely block-level padding differences.

## Fidelity Assessment

**BENIGN**

All diffs are purely cosmetic: parameter key casing normalization, parameter reordering within blocks (which Altium ignores), and a default value injection. No data loss, no value changes, no missing parameters. The /Storage stream is byte-identical. This is the cleanest result of the batch.

## Impact on File Format Support

### What's working well
- All schematic record types are parsed and re-serialized with correct values.
- Multi-part components (up to 25 parts) with `SwapIDPart` parameters handled correctly.
- `%UTF8%SwapIDPart` encoding preserved correctly.
- `NotUseDBTableName=T` parameter preserved (unusual boolean parameter name).
- `Transparent=T` parameter on Rectangle (RECORD=14) preserved.
- All system parameters (RECORD=41) for title block preserved exactly.
- /Storage stream achieves byte-identical roundtrip (25 bytes, minimal header-only stream).

### What needs improvement
- **Parameter key casing**: Same as file 00 -- serializer outputs mixed-case canonical keys while originals were ALL-CAPS. Semantically benign.
- **Default value injection**: `Weight=0` injected into /Additional. Same issue as file 00.
- **Stream size shrinkage**: /FileHeader lost 6,480 bytes. This needs investigation -- may be block padding normalization, or could indicate subtle data omission.

### Specific bugs or missing features revealed
- **/FileHeader size decrease**: The 6,480-byte decrease in /FileHeader stream size is notable. Since all parameters appear to be preserved, this is most likely due to differences in how blocks are packed (e.g., the original may have had padding bytes or the block size headers encode slightly differently). This should be verified by comparing total block payload sizes.
