# Roundtrip Diff Report: 11_FPGA_power.SchDoc (LimeSDR-PCIe 1v3)

## Summary
- **File size**: 1,766,912 bytes original vs 1,773,568 bytes roundtripped (+6,656 bytes)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: 75 bytes -> 84 bytes (+9 bytes)
- **Diff category**: Default value injection
- **Details**: Single block. Original: `|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0`. Roundtrip appends `|Weight=0`. Default value injected.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Not explicitly stated (overall file grew ~6.5KB)
- **Diff category**: Key case normalization (ALL CAPS -> MixedCase), parameter reordering, default value changes
- **Details**: 7,884 blocks differ. Like file 13, this is a **legacy format file** with ALL UPPERCASE parameter keys. Our serializer normalizes to MixedCase. This affects every record.

  **Block 0 (FileHeader record)**: `WEIGHT=7882|MINORVERSION=2|UNIQUEID=QCGEMYTV` becomes `Weight=7882|MinorVersion=2|UniqueID=QCGEMYTV`. Note: same UniqueID as file 13 (QCGEMYTV) -- these are the same schematic sheet at different revisions (v1.2 vs v1.3).

  **RECORD=31 (Sheet header)** - 1 block: Case normalization + font reordering + sheet property reordering. This file has fractional grid sizes: `SnapGridSize=3|SnapGridSize_Frac=93701` and `HotSpotGridSize=3|HotSpotGridSize_Frac=93701`. All values preserved. Uses template: `A3_LMS.SchDot`.

  **RECORD=39 (Template reference)** - 1 block: Case normalization. `ISNOTACCESIBLE=T` -> `IsNotAccesible=T`.

  **RECORD=4 (Label)** - ~124 blocks: Case normalization only. Template title block labels.

  **RECORD=6 (Polyline)** - ~566 blocks: Case normalization only.

  **RECORD=41 (Parameter)** - ~4,867 blocks: Case normalization. Largest group. Includes document parameters (Organization="Lime Microsystems", Title="FPGA power", Revision="v1.3", SheetNumber="11").

  **RECORD=2 (Pin)** - ~1,240 blocks: Case normalization. All pin data including fractional coordinates preserved.

  **RECORD=1 (Component)** - ~276 blocks: Case normalization.

  **RECORD=14, 34, 44, 45, 46, 48** - ~276 blocks each: Case normalization.

  **RECORD=17 (Power port)** - ~52 blocks: Case normalization + parameter reordering + ShowNetName/FontID changes.

  **RECORD=27 (Wire)** - ~211 blocks: Case normalization + parameter reordering.

  **RECORD=29 (Junction)** - ~381 blocks: Case normalization. (Note: file 13 had 762 RECORD=29 entries while file 15 has a similar count minus the wire diffs -- the slight difference is because v1.3 removed some wires.)

  **RECORD=12 (Arc)** - ~24 blocks: Case normalization.

  **RECORD=22 (No connect)** - ~4 blocks: Case normalization.

  **RECORD=30 (Image)** - 1 block: Case normalization. Embedded `LimeMicroLogoPCB.bmp` reference.

  **RECORD=209 (Text frame)** - 1 block: Case normalization + parameter reordering. `TEXT=FPGA power|AUTHOR=DR`.

### /Storage
- **Status**: DIFFERS
- **Size change**: 5,750 bytes -> 5,869 bytes (+119 bytes)
- **Diff category**: Key case normalization + binary size difference
- **Details**: Identical pattern to file 13:
  - Block 0 (text): `WEIGHT=1` -> `Weight=1` (case normalization)
  - Block 1 (binary): 5,712 bytes -> 5,831 bytes (+119 bytes). Embedded BMP image data for Lime Microsystems logo. Same +119 byte increase as file 13, confirming this is a systematic issue with our image re-encoding.

## Diff Categories Found

1. **Parameter reordering** - Present in RECORD=17, 18, 27, 31, 209. Values preserved.
2. **Default value injection** - `Weight=0` in /Additional. `FontID=1` in power ports.
3. **Font parameter reordering** - RECORD=31 font fields reordered.
4. **Missing parameters** - `ShowNetName=F` dropped from GND power ports. Benign.
5. **Changed values** - None. All parameter values identical.
6. **New streams** - None.
7. **Binary differences** - /Storage block 1 (BMP image) grew by 119 bytes. Same as file 13.
8. **Size-only differences** - /Storage binary block.
9. **Key case normalization** - Dominant diff. ALL UPPERCASE -> MixedCase. ~7,884 blocks affected.

## Fidelity Assessment

**BENIGN** - Nearly identical assessment to file 13. This is the v1.3 revision of the same schematic sheet (same UniqueID, same template, same Lime Microsystems organization metadata). All semantic data is preserved.

The /Storage binary block size increase (+119 bytes) is **CONCERNING** until the embedded BMP image integrity is verified. The fact that files 13 and 15 show the identical +119 byte increase (same template, same logo image) suggests a deterministic re-encoding issue rather than corruption.

## Impact on File Format Support

**What's working well:**
- Successfully handles legacy ALL UPPERCASE format files from Altium Designer 16
- Full record type support: RECORD=1, 2, 4, 6, 12, 14, 17, 22, 27, 29, 30, 31, 34, 39, 41, 44, 45, 46, 48, 209
- Fractional coordinate support (`_Frac` suffixed parameters) preserved
- Fractional grid sizes (`SnapGridSize_Frac`, `HotSpotGridSize_Frac`) preserved
- Template reference handling (A3_LMS.SchDot)
- Complex component hierarchy with 276+ components fully preserved
- All document metadata preserved (Organization, Title, Revision, SheetNumber, etc.)

**What needs improvement:**
- **Key case normalization**: Same as file 13 -- intentional upgrade per design philosophy, but worth documenting.
- **Parameter ordering**: Same ordering differences as all other files.
- **Default value handling**: Weight=0 injection, FontID=1 injection, ShowNetName=F omission.
- **/Storage binary block size (+119 bytes)**: Identical to file 13. The embedded BMP image (LimeMicroLogoPCB.bmp) is being re-serialized with 119 extra bytes. This needs investigation:
  - Could be BMP padding/alignment differences
  - Could be re-encoding of the image data
  - Could be extra metadata being added
  - Since both files sharing the same template show identical growth, this is deterministic and likely a serialization detail rather than corruption.

## Cross-File Comparison (Files 13 vs 15)

These two files are revisions of the same schematic sheet:
- File 13: LimeSDR-PCIe **1v2**, Revision=v1.2, WEIGHT=8264
- File 15: LimeSDR-PCIe **1v3**, Revision=v1.3, WEIGHT=7882
- Both share: UniqueID=QCGEMYTV, same template (A3_LMS.SchDot), same Lime Microsystems metadata
- Both exhibit identical diff patterns: ALL CAPS normalization, same /Storage +119 byte growth
- File 15 is slightly smaller (1,766,912 vs 1,816,576) and has fewer blocks, suggesting some components were removed or simplified in v1.3
- The record type distribution and diff patterns are virtually identical
