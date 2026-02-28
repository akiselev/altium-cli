# Roundtrip Diff Report: PwrAndBias.SchDoc (LimeSDR-XTRX rev5)

## Summary
- **File size**: 1,840,640 bytes original vs 1,843,200 bytes roundtripped (+2,560 bytes)
- **Save-as result**: Success
- **Streams differing**: 2 (/Additional, /FileHeader)
- **Streams identical**: 1 (/Storage)

## Stream-by-Stream Analysis

### /Additional
- **Status**: DIFFERS
- **Size change**: 75 bytes -> 84 bytes (+9 bytes)
- **Diff category**: Default value injection
- **Details**: Single block. Original: `|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0`. Roundtrip appends `|Weight=0`. This is a default value injected by our serializer.

### /FileHeader
- **Status**: DIFFERS
- **Size change**: 1,799,708 bytes -> 1,799,642 bytes (-66 bytes)
- **Diff category**: Parameter reordering, default value changes (ShowNetName removal), FontID injection
- **Details**: 413 blocks differ across the following record types:

  **RECORD=31 (Sheet header)** - 1 block: Font parameters reordered (FontName before Size). Sheet-level properties reordered. `SheetStyle=1` (A-size) with `CustomX=1550|CustomY=1110`. All values preserved.

  **RECORD=17 (Power port)** - 156 blocks: Same patterns as file 12:
  - Style=2 (GND) ports: `Style=2|ShowNetName=F` replaced with `Style=2`, `ShowNetName=F` dropped, `FontID=1` added. ~105 blocks lose ShowNetName.
  - Style=1 (named power) ports: `Style=1|ShowNetName=T` moved to end, `FontID=1` added.
  - Parameters reordered: Location/Color/Text first, style/orientation later.
  - Net names include: GND, +1.2V_MGTAVTT, +1.0V_MGTAVCC, +1.8V_VCCAUX, +1.0V_VCCINT, +1.8V_VDLMS, +1.8V_VALMS, +1.2V_VDLMS, +3,3V, +3.3/5VIN, +1.8/3.3VCCIO, +1.4V_VALMS, +1.25V_VALMS, +3V_VACLK, +2.05V, +1.5V, +1.75V, +1.8/3.3V

  **RECORD=18 (Sheet entry)** - 6 blocks: Parameters reordered. `Alignment`, `Width`, `Height` moved after Name. `IOType` moved. All values identical.

  **RECORD=27 (Wire)** - 250 blocks: Parameter reordering only. `LineWidth|Color|UniqueID|LocationCount` becomes `Color|LineWidth|LocationCount|...|UniqueID`. All values identical, same byte lengths.

### /Storage
- **Status**: OK (identical, 25 bytes)

## Diff Categories Found

1. **Parameter reordering** - Present in all record types (RECORD=17, 18, 27, 31). All values preserved. Dominant diff type.
2. **Default value injection** - `Weight=0` added to /Additional header. `FontID=1` added to RECORD=17 power port records.
3. **Font parameter reordering** - RECORD=31 font fields reordered (FontName before Size).
4. **Missing parameters** - `ShowNetName=F` dropped from Style=2 (GND) power ports (~105 instances). Benign default.
5. **Changed values** - None detected. All parameter values are semantically identical.
6. **New streams** - None.
7. **Binary differences** - None.
8. **Size-only differences** - None.
9. **Other** - None.

## Fidelity Assessment

**BENIGN** - All semantic data is preserved. The differences are purely:
- Parameter key ordering (Altium is order-insensitive)
- Default value handling (FontID=1 injection, ShowNetName=F omission)

The -66 byte size change in /FileHeader is accounted for by dropping `ShowNetName=F` from ~105 GND power ports, partially offset by adding `FontID=1` to all power ports.

This file is very similar to file 12 (same project, same revision) and exhibits identical diff patterns with no additional issues.

## Impact on File Format Support

**What's working well:**
- All record types parsed and serialized correctly (RECORD=17, 18, 27, 31)
- Wide variety of power net names handled correctly (including comma-separated voltage names like `+3,3V`, slash-separated like `+3.3/5VIN` and `+1.8/3.3VCCIO`)
- Fractional coordinates not present in this file (simpler coordinate model)
- /Storage stream byte-identical roundtrip
- No encoding issues (no non-ASCII characters in this file)

**What needs improvement:**
- **Parameter ordering**: Same as other files -- our serializer outputs in a different order than Altium's native serializer.
- **Default value handling for RECORD=17**: FontID=1 injection and ShowNetName=F omission. Should match Altium's behavior.
- **Weight=0 injection**: /Additional header should not append `Weight=0` when the original did not have it.
