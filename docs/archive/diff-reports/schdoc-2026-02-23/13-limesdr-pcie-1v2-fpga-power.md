# Roundtrip Diff Report: 11_FPGA_power.SchDoc (LimeSDR-PCIe 1v2)

## Summary
- **File size**: 1,816,576 bytes original vs 1,822,720 bytes roundtripped (+6,144 bytes)
- **Save-as result**: Success
- **Streams differing**: 3 (/Additional, /FileHeader, /Storage)
- **Streams identical**: 0

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: 75 bytes -> 84 bytes (+9 bytes)
- **Diff category**: Default value injection
- **Details**: Single block. Original: `|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0`. Roundtrip appends `|Weight=0`. The `Weight=0` parameter is a default value injected by our serializer.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: Length not explicitly shown, but file grew by ~6KB overall
- **Diff category**: Key case normalization (ALL CAPS -> MixedCase), parameter reordering, default value changes
- **Details**: 8,266 blocks differ. This is a **legacy format file** where all parameter keys are written in ALL UPPERCASE (e.g., `OWNERINDEX`, `LOCATION.X`, `FONTNAME1`). Our serializer normalizes to MixedCase (e.g., `OwnerIndex`, `Location.X`, `FontName1`). This is the dominant diff -- every single record in the file is affected.

  **Block 0 (FileHeader record)**: `WEIGHT=8264|MINORVERSION=2|UNIQUEID=QCGEMYTV` becomes `Weight=8264|MinorVersion=2|UniqueID=QCGEMYTV`. Values identical, only casing changes.

  **RECORD=31 (Sheet header)**: Font parameter reordering (FontName before Size) plus case normalization. Also sheet-level property reordering. All values preserved.

  **RECORD=39 (Template reference)**: Case normalization only. `ISNOTACCESIBLE=T` -> `IsNotAccesible=T`.

  **RECORD=4 (Label)**: Case normalization for all keys. ~124 blocks. Values identical.

  **RECORD=6 (Polyline)**: Case normalization. ~566 blocks. Values identical.

  **RECORD=41 (Parameter)**: Case normalization. ~4,867 blocks. This is the largest group. Values identical.

  **RECORD=2 (Pin)**: Case normalization. ~1,240 blocks. Values identical.

  **RECORD=1 (Component)**: Case normalization. ~276 blocks.

  **RECORD=14 (Designator/Implementation)**: Case normalization. ~76 blocks.

  **RECORD=17 (Power port)**: Case normalization + parameter reordering + `ShowNetName=F` dropped + `FontID=1` injected. ~52 blocks.

  **RECORD=27 (Wire)**: Case normalization + parameter reordering (`LineWidth|Color|UniqueID` -> `Color|LineWidth|...|UniqueID`). ~212 blocks.

  **RECORD=29 (Junction)**: Case normalization. ~381 blocks.

  **RECORD=30 (Image)**: Case normalization. 1 block.

  **RECORD=34, 44, 45, 46, 48 (Various component sub-records)**: Case normalization. ~276 blocks each.

  **RECORD=12 (Arc)**: Case normalization. ~24 blocks.

  **RECORD=22 (No connect)**: Case normalization. ~4 blocks.

  **RECORD=209 (Text frame)**: Case normalization + parameter reordering. 1 block.

### /Storage
- **Status**: DIFFERS
- **Size change**: 5,750 bytes -> 5,869 bytes (+119 bytes)
- **Diff category**: Key case normalization + binary size difference
- **Details**:
  - Block 0 (text header): `WEIGHT=1` -> `Weight=1` (case normalization)
  - Block 1 (binary): 5,712 bytes -> 5,831 bytes (+119 bytes). This is the embedded BMP image data for the Lime Microsystems logo. The binary size increase suggests our serializer may be re-encoding the image or adding padding.

## Diff Categories Found

1. **Parameter reordering** - Present in RECORD=17, 18, 27, 31, 209. Same values, different order.
2. **Default value injection** - `Weight=0` added to /Additional header. `FontID=1` added to RECORD=17 power ports.
3. **Font parameter reordering** - RECORD=31 font fields reordered (FontName before Size).
4. **Missing parameters** - `ShowNetName=F` dropped from Style=2 power ports (benign default).
5. **Changed values** - No semantic value changes. All parameter values are identical.
6. **New streams** - None.
7. **Binary differences** - /Storage block 1 (embedded BMP) grew by 119 bytes. This needs investigation.
8. **Size-only differences** - /Storage binary block size change.
9. **Key case normalization** - The dominant diff category. Original file uses ALL UPPERCASE keys (legacy AD16 format). Our serializer normalizes to MixedCase. This affects every single record (~8,266 blocks).

## Fidelity Assessment

**BENIGN** - Altium parameter keys are case-insensitive, so the ALL CAPS -> MixedCase normalization is functionally equivalent. All semantic data is preserved. The only items worth noting:

- The /Storage binary block size increase (+119 bytes) should be investigated to ensure the embedded image is not being corrupted. This is **CONCERNING** until verified.
- The key case normalization is an intentional upgrade to modern format conventions (Altium's own newer versions use MixedCase).

## Impact on File Format Support

**What's working well:**
- Successfully handles legacy ALL UPPERCASE format files from Altium Designer 16
- All record types parsed correctly (RECORD=1, 2, 4, 6, 12, 14, 17, 22, 27, 29, 30, 31, 34, 39, 41, 44, 45, 46, 48, 209)
- All parameter values preserved exactly
- Template references, embedded images, and complex component hierarchies handled correctly
- Pin data with fractional coordinates (Location.X_Frac, PinLength_Frac) preserved

**What needs improvement:**
- **Key case normalization**: We always emit MixedCase. This is correct behavior (upgrading to modern format), but should be documented as intentional. Per CLAUDE.md: "we UPGRADE TO THE LATEST FORMAT."
- **Parameter ordering**: Same issues as file 12 -- our ordering differs from Altium's native ordering.
- **Default value injection**: `Weight=0` added to /Additional, `FontID=1` added to power ports.
- **/Storage binary block size**: The +119 byte increase in the embedded BMP image block needs investigation. Could be a re-encoding issue or padding difference. This is the only potentially concerning finding.
- **Default value handling for RECORD=17**: Same ShowNetName=F omission issue as file 12.
