# Roundtrip Diff Report: hekkelek_amiga-keyboard__hw__keyboard_matrix_full.SchDoc

## Summary
- **File size**: 1,946,624 bytes original -> 1,888,256 bytes roundtripped (-58,368 bytes / -3.0%)
- **Save-as result**: Success
- **Streams differing**: 2 (/Additional, /FileHeader)
- **Streams identical**: 1 (/Storage)

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: 75 bytes -> 84 bytes (+9 bytes)
- **Diff category**: Default value injection
- **Details**: 1 block differs (block[0]). The roundtrip adds `Weight=0` to the header record:
  - Original: `|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0`
  - Roundtrip: `|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0|Weight=0`

### /FileHeader
- **Status**: DIFFERS
- **Size change**: 1,928,713 bytes -> 1,865,033 bytes (-63,680 bytes / -3.3%)
- **Diff category**: Case normalization, duplicate parameter removal, parameter reordering, default value injection
- **Details**: 11,372 blocks differ (out of ~11,372 total). This is a legacy-format file where the original uses ALL-UPPERCASE parameter keys. Every single record is affected. Key patterns:

  **1. Case normalization (ALL blocks):**
  The original file uses uppercase parameter keys (`RECORD`, `OWNERPARTID`, `LOCATION.X`, `FONTID`, etc.). Our serializer writes canonical mixed-case (`Record` is kept as `RECORD`, but `OwnerPartId`, `Location.X`, `FontID`, etc.). This accounts for the vast majority of diffs and is the primary cause of the size reduction (shorter key names when case is normalized, plus removal of duplicate keys).

  **2. Duplicate parameter removal (RECORD=41 blocks):**
  The original file contains duplicate `ISHIDDEN=T` parameters in RECORD=41 (Template Parameter) records. For example:
  - Original: `...ISHIDDEN=T|TEXT=*|ISHIDDEN=T|NAME=CurrentTime...`
  - Roundtrip: `...IsHidden=T|Text=*|Name=CurrentTime...`
  The second `ISHIDDEN=T` is removed. Since Altium uses first-occurrence-wins, this is correct -- the duplicate was redundant. However, this means the roundtrip output is shorter, contributing to the -63KB size decrease.

  **3. Default value injection (block[0]):**
  The header block gains `MinorVersion=0` and `UniqueID=` (empty):
  - Original: `|HEADER=...|WEIGHT=11371`
  - Roundtrip: `|HEADER=...|Weight=11371|MinorVersion=0|UniqueID=`

  **4. Duplicate parameter removal in RECORD=31 (block[1]):**
  The original has a duplicate `HOTSPOTGRIDON=T`:
  - Original: `...HOTSPOTGRIDON=T|HOTSPOTGRIDON=T|HOTSPOTGRIDSIZE=4...`
  - Roundtrip: `...HotSpotGridOn=T|HotSpotGridSize=4...`

  **5. Font parameter reordering (block[1]):**
  Same pattern as files 08/09: font fields reordered from `SizeN|FontNameN` to `FontNameN|SizeN`.

  **6. Sheet property reordering (block[1]):**
  Grid, sheet style, border, and display properties reordered to our canonical order.

### /Storage
- **Status**: OK
- **Size change**: 25 bytes (identical)
- **Details**: Byte-identical. This file has a minimal Storage stream.

## Diff Categories Found

1. **Parameter reordering** -- Present in every block. Font fields, sheet properties, and record-specific fields all reordered to our canonical order.
2. **Default value injection** -- `Weight=0` added to /Additional header. `MinorVersion=0` and `UniqueID=` added to /FileHeader header.
3. **Font parameter reordering** -- RECORD=31 font fields reordered.
4. **Missing parameters** -- Duplicate `ISHIDDEN=T` removed from ~2,976 RECORD=41 blocks. Duplicate `HOTSPOTGRIDON=T` removed from RECORD=31. **Note:** These are duplicate removals, not data loss. The removed parameters are exact duplicates whose second occurrence would be ignored by Altium's first-occurrence-wins parser. Semantically benign.
5. **Case normalization** -- All ~11,372 blocks have parameter keys converted from ALL-UPPERCASE to canonical mixed-case. This is the format upgrade behavior (legacy -> modern).

## Fidelity Assessment

**BENIGN**: Despite the dramatic -3.0% size decrease, all changes are semantically invisible to Altium:
- Case normalization: Altium parameter lookup is case-insensitive
- Duplicate removal: Second occurrences are ignored by first-occurrence-wins
- Default value injection: `Weight=0`, `MinorVersion=0`, empty `UniqueID` are default values
- Parameter reordering: Order does not matter for Altium parsing

This file demonstrates the "upgrade to latest format" behavior described in the project docs -- legacy files with uppercase keys are normalized to modern mixed-case on save.

## Impact on File Format Support

**What's working well:**
- Complete parsing of a legacy ALL-UPPERCASE format file
- Correct handling of duplicate parameters (removed safely)
- All record types fully supported: 1, 2, 4, 6, 29, 31, 34, 39, 41, 44, 45, 46, 48
- /Storage stream preserved byte-identical
- Every parameter value preserved correctly

**What needs improvement:**
- The `MinorVersion=0` and empty `UniqueID=` injection in the header may not be desired for files that originally lacked these fields. This should be verified against Altium's save behavior.
- The `Weight=0` injection in /Additional should similarly be verified.
- Parameter ordering for font fields and sheet properties could be made to match Altium's native order for closer-to-byte-identical output.
- The -63KB size decrease from duplicate removal and case normalization is expected upgrade behavior, but worth documenting for users who may be surprised by file size changes.
